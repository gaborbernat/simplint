use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use log::{debug, error};
use rayon::prelude::*;
use ruff_linter::package::PackageRoot;
use ruff_linter::packaging::detect_package_root;
use ruff_linter::{SuppressionKind, warn_user_once};
use rustc_hash::FxHashMap;

use super::digest::run_digest;
use super::discover::{Discovered, discover, linted_roots, lints_stdin};
use super::files::{Documentation, Plan, ProjectInputs, RunFile, documentation};
use super::rounds::{DiscoveredFile, file_facts, index_and_lint};
use super::{
    Diagnostics, Mode, Overlay, Resolved, StdinLint, io_error, lint_path, lint_stdin, reports_io_errors, warn_missing,
    write_source,
};
use crate::project::cache::{self, FileKey};
use crate::project::{DocReferences, Ending, External, FileFacts, Parts, SearchRoots};
use crate::run::cache::{Cache, Caches, Fingerprints, FullDisplay};
use crate::settings::Settings;

// Ruff's `add_noqa`: a file that cannot be read or written is logged and left as it is.
pub(crate) fn add_suppressions(
    files: &[PathBuf],
    resolved: &Resolved,
    kind: SuppressionKind,
    reason: Option<&str>,
) -> Result<usize> {
    // A suppression comment changes no file's facts, so one round is final.
    let roots = linted_roots(files);
    let (counts, missing) = for_each_file(
        files,
        resolved,
        &Plumbing::plain(Ending::Drop, &Overlay::default(), &resolved.cwd),
        |DiscoveredFile { path, linter, .. }| {
            let source = match std::fs::read_to_string(path) {
                Ok(source) => source,
                Err(error) => {
                    error!("Failed to extract source from {}: {error}", path.display());
                    return 0;
                }
            };
            let (count, output) = linter.add_suppressions(path, &source, kind, reason);
            if count > 0
                && let Err(error) = write_source(path, &roots, &source, &output)
            {
                error!("Failed to add suppression to {}: {error}", path.display());
                return 0;
            }
            count
        },
    )?;
    warn_missing(&missing);
    Ok(counts.into_iter().flatten().flatten().sum())
}

pub(crate) struct CacheUse {
    enabled: bool,
    full: Option<FullDisplay>,
}

impl CacheUse {
    // As in Ruff, the cache is only read and written while `noqa` comments apply.
    pub(crate) fn new(no_cache: bool, mode: &Mode, full: Option<FullDisplay>) -> Self {
        let enabled = !no_cache && mode.noqa.is_enabled();
        // Only generated fixes keep their reports in the cache, so only then can the `full` text be kept with them.
        let full = full.filter(|_| enabled && mode.fix.is_generate());
        Self { enabled, full }
    }
}

pub(crate) fn check_files(
    files: &[PathBuf],
    resolved: &Resolved,
    mode: &Mode,
    usage: &CacheUse,
    ending: Ending,
) -> Result<Diagnostics> {
    let full = usage.full.as_ref();
    let fingerprints = Fingerprints::default();
    let overlay = Overlay::default();
    // Applied fixes reach the disk, and a diff run keeps its fixes in the overlay; either way other files' facts come
    // from them.
    let rewrote = |diagnostics: &Diagnostics| !diagnostics.fixed.is_empty();
    // Without fixes a file's reports need no index once the run's digest matches the one they were stored under.
    let reuse = |file: &RunFile, cache: &Cache, key: FileKey| {
        let base = fingerprints.base(&file.identity());
        cache.reused(file.path, key, base, full)
    };
    let reuse = mode.fix.is_generate().then_some(&reuse as &Reuse<'_, Diagnostics>);
    let plumbing = Plumbing {
        cache_dir: resolved.cache_dir.as_deref().filter(|_| usage.enabled),
        rewrote: (!mode.fix.is_generate()).then_some(&rewrote),
        reuse,
        ending,
        overlay: &overlay,
        cwd: &resolved.cwd,
    };
    let (results, missing) = for_each_file(files, resolved, &plumbing, |file| {
        let lint = |settled| lint_path(file.path, &file.linter, mode, settled, &overlay);
        let Some((cache, key)) = file.cache else {
            return lint(None).diagnostics;
        };
        cache.lint_or_reuse(file.path, key, fingerprints.of(&file.linter, mode), full, lint)
    })?;
    let file_results = results.into_iter().flatten().filter_map(|passes| passes.into_iter().reduce(Diagnostics::then));
    Ok(file_results.fold(Diagnostics::default(), |left, right| left + right) + missing_diagnostics(missing, resolved))
}

// Ruff's `io-error` for each path the run names that does not exist, or a warning each while that rule is off.
fn missing_diagnostics(missing: Vec<(PathBuf, String)>, resolved: &Resolved) -> Diagnostics {
    if !reports_io_errors(&resolved.pyproject_config.settings.linter) {
        warn_missing(&missing);
        return Diagnostics::default();
    }
    Diagnostics {
        inner: missing.into_iter().map(|(path, message)| io_error(&path, message)).collect(),
        ..Diagnostics::default()
    }
}

// Ruff's `check_stdin`: the source comes from standard input, and nothing else is indexed. Otherwise it is linted as
// the file it stands for would be, so its settings, package and search roots come from that file's absolute path. A
// name the file filters leave out reports nothing, and with `--fix` its source goes out unchanged, as Ruff writes it.
pub(crate) fn check_stdin(path: &Path, source: &str, resolved: &Resolved, mode: &Mode) -> Result<StdinLint> {
    let absolute = std::path::absolute(path).expect("clap rejects an empty `--stdin-filename`");
    if !lints_stdin(&absolute, resolved)? {
        return Ok(StdinLint {
            diagnostics: Diagnostics::default(),
            output: mode.fix.is_apply().then(|| source.to_owned()),
        });
    }
    lint_alone(&absolute, source, resolved, |file| lint_stdin(path, source, &file.linter, mode))
}

// A file linted on its own, against an index of that file alone, as standard input is.
fn lint_alone<T: Send>(
    absolute: &Path,
    source: &str,
    resolved: &Resolved,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<T> {
    let ruff = &resolved.pyproject_config.settings;
    let settings = resolved.settings_resolver.for_file(absolute)?;
    let package = absolute
        .parent()
        .and_then(|parent| detect_package_root(parent, &ruff.linter.namespace_packages))
        .map(PackageRoot::root);
    let roots = SearchRoots::new(ruff, package);
    let files = [RunFile::new(absolute, &settings, ruff, resolved.pyproject_config.path.as_deref(), package, &roots)];
    let linted: Vec<&RunFile> = files.iter().collect();
    let Plan { rules, parts } = Plan::of(&linted, resolved)?;
    let docs = documentation(&linted, &resolved.settings_resolver.root.root, rules, None)
        .remove(&files[0].nested)
        .unwrap_or_default();
    let facts =
        FileFacts::collect(absolute, source, files[0].module_path.as_deref(), files[0].importable.as_deref(), parts);
    let mut passes = index_and_lint(
        &files,
        (parts, &ProjectInputs::of(&files, &roots), &roots.startup_names()),
        &roots,
        vec![facts],
        &docs,
        &External::default(),
        &[None],
        &Plumbing::plain(Ending::Drop, &Overlay::default(), &resolved.cwd),
        &work,
    )?;
    Ok(passes.pop().and_then(|mut file| file.pop()).expect("the file is linted once"))
}

// Hands `work` the linter of a module of `source` in a project of its own, set up by the `check` arguments `args`, for
// tests of what runs on a linter.
#[cfg(test)]
pub(crate) fn with_linter<T: Send>(
    source: &str,
    args: &[&str],
    work: impl Fn(&crate::lint::Linter, &Path) -> T + Sync,
) -> T {
    use clap::{Args, FromArgMatches};

    use crate::cli::args::{CheckCommand, GlobalConfigArgs};

    let directory = tempfile::TempDir::new().expect("temporary directory");
    let path = directory.path().join("module.py");
    let matches = GlobalConfigArgs::augment_args(CheckCommand::augment_args(clap::Command::new("check")))
        .get_matches_from([&["check", "--isolated", "--no-cache"], args].concat());
    let check = CheckCommand::from_arg_matches(&matches).expect("the check arguments parse");
    let global_options = GlobalConfigArgs::from_arg_matches(&matches).expect("the global arguments parse");
    let resolved = Resolved::new(&check, &global_options).expect("the isolated settings resolve");
    lint_alone(&path, source, &resolved, |file| work(&file.linter, &path)).expect("the module lints")
}

// Ruff's per-file loop over the discovered files, with the import index of the whole run built first, and the caches
// in `plumbing.cache_dir` read before and written after. With `rewrote` set, the files a round rewrites are indexed and
// linted again, as `lint_rounds` tells. The paths the run names that do not exist come back with the passes, for the
// caller to report.
fn for_each_file<T: Send>(
    files: &[PathBuf],
    resolved: &Resolved,
    plumbing: &Plumbing<'_, T>,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Walked<T>> {
    let start = Instant::now();
    let Discovered { python, unlinted, resolver, missing } = discover(files, resolved)?;
    debug!("Identified files to lint in: {:?}", start.elapsed());
    if python.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
        return Ok((None, missing));
    }
    let paths: Vec<&Path> = python.iter().chain(&unlinted).map(PathBuf::as_path).collect();
    let settings: Vec<Arc<Settings>> =
        paths.iter().map(|path| resolved.settings_resolver.for_file(path)).collect::<Result<_>>()?;
    let packages = resolver.package_roots(&paths);
    let roots = SearchRoots::new(&resolved.pyproject_config.settings, packages.values().flatten().copied());
    // Each nested project is indexed and linted on its own, so the files are grouped by project, and kept in the run's
    // order within a group, where the linted ones come first.
    let mut indexed: Vec<RunFile> = paths
        .iter()
        .zip(&settings)
        .enumerate()
        .map(|(at, (&path, settings))| {
            let (ruff, ruff_config) = resolver.resolve_with_path(path);
            let package = path.parent().and_then(|parent| packages.get(parent)).copied().flatten();
            RunFile {
                at,
                linted: at < python.len(),
                ..RunFile::new(path, settings, ruff, ruff_config, package, &roots)
            }
        })
        .collect();
    indexed.sort_by(|left, right| (&left.nested, left.at).cmp(&(&right.nested, right.at)));
    let linted: Vec<&RunFile> = indexed.iter().filter(|file| file.linted).collect();
    let Plan { rules, parts } = Plan::of(&linted, resolved)?;
    if let Some(cache_dir) = plumbing.cache_dir {
        cache::prepare(cache_dir);
    }
    let docs = || documentation(&linted, &resolved.settings_resolver.root.root, rules, plumbing.cache_dir);
    let passes = index_cached(&indexed, parts, (&roots, &packages), docs, plumbing, work)?;
    Ok((passes, missing))
}

// The caches of the run opened, then either every linted file answered from them under the run's digest, or the facts
// of the files read from them or collected, and the files indexed and linted project by project. `docs` gives what the
// documentation names. The documentation walk reads files of its own, so it runs while the caches open.
fn index_cached<'f, T: Send>(
    indexed: &'f [RunFile<'f>],
    parts: Parts,
    (roots, packages): (&SearchRoots, &FxHashMap<&'f Path, Option<PackageRoot<'f>>>),
    docs: impl FnOnce() -> Documentation + Send,
    plumbing: &Plumbing<'_, T>,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Option<Vec<Vec<T>>>> {
    let (caches, docs) = rayon::join(
        || {
            plumbing.cache_dir.map(|cache_dir| {
                Caches::open(
                    cache_dir,
                    packages.iter().map(|(parent, package)| package.map_or(*parent, PackageRoot::path)),
                )
            })
        },
        docs,
    );
    let startup = roots.startup_names();
    let groups: Vec<&[RunFile]> = indexed.chunk_by(|left, right| left.nested == right.nested).collect();
    let inputs: Vec<ProjectInputs> = groups.iter().map(|group| ProjectInputs::of(group, roots)).collect();
    let linted = indexed.iter().filter(|file| file.linted).count();
    // Only the cache reads the digest, which stats every file and walks every directory an import can name, so a run
    // without one skips it.
    let opened = caches.as_ref().map(|caches| {
        let keys: Vec<Option<FileKey>> = indexed.par_iter().map(|file| FileKey::of(file.path)).collect();
        let run = run_digest(indexed, &keys, linted, roots, &startup, &inputs, &docs, parts);
        (caches.views(run), keys)
    });
    // As in Ruff, a file's cache is its package's, or its directory's outside a package.
    let file_caches: Vec<Option<(&Cache, FileKey)>> = opened.as_ref().map_or_else(
        || vec![None; indexed.len()],
        |(views, keys)| {
            indexed
                .iter()
                .zip(keys)
                .map(|(file, key)| {
                    let parent = file.path.parent().expect("a discovered file lies in a directory");
                    Some((&views[file.package.map_or(parent, PackageRoot::path)], (*key)?))
                })
                .collect()
        },
    );
    if let Some(reuse) = plumbing.reuse {
        let reused: Option<Vec<(usize, Vec<T>)>> = indexed
            .par_iter()
            .zip(&file_caches)
            .filter(|(file, _)| file.linted)
            .map(|(file, cached)| {
                let (cache, key) = (*cached)?;
                Some((file.at, vec![reuse(file, cache, key)?]))
            })
            .collect();
        if let Some(passes) = reused {
            persist_views(opened.map(|(views, _)| views), plumbing.ending)?;
            return Ok(Some(in_file_order(passes)));
        }
    }
    let mut facts: Vec<Option<FileFacts>> = indexed
        .par_iter()
        .zip(&file_caches)
        .map(|(file, cached)| file_facts(file, *cached, parts, plumbing.overlay))
        .collect();
    let mut passes: Vec<(usize, Vec<T>)> = Vec::new();
    let no_docs = DocReferences::default();
    let no_external = External::default();
    let external = if indexed.iter().any(|file| file.nested.is_some()) {
        outer_names(indexed, &facts)
    } else {
        External::default()
    };
    let mut remaining = &file_caches[..];
    for (project, project_inputs) in groups.iter().zip(&inputs) {
        let (project_caches, rest) = remaining.split_at(project.len());
        remaining = rest;
        let project_passes = index_and_lint(
            project,
            (parts, project_inputs, &startup),
            roots,
            facts.drain(..project.len()).collect(),
            docs.get(&project[0].nested).unwrap_or(&no_docs),
            if project[0].nested.is_some() { &external } else { &no_external },
            project_caches,
            plumbing,
            &work,
        )?;
        passes.extend(project.iter().filter(|file| file.linted).map(|file| file.at).zip(project_passes));
    }
    persist_views(opened.map(|(views, _)| views), plumbing.ending)?;
    Ok(Some(in_file_order(passes)))
}

// Writes the caches the run changed, each package's on its own thread.
fn persist_views(views: Option<FxHashMap<&Path, Cache>>, ending: Ending) -> Result<()> {
    let Some(views) = views else {
        return Ok(());
    };
    views.into_values().collect::<Vec<_>>().into_par_iter().try_for_each(|cache| cache.persist(ending))
}

// The passes of the linted files, in the order the run lists them.
fn in_file_order<T>(mut passes: Vec<(usize, T)>) -> Vec<T> {
    passes.sort_unstable_by_key(|(at, _)| *at);
    passes.into_iter().map(|(_, pass)| pass).collect()
}

// What the files outside every nested project read, which the files of a nested project count as used by.
fn outer_names(indexed: &[RunFile], facts: &[Option<FileFacts>]) -> External {
    let mut external = External::default();
    for facts in
        indexed.iter().zip(facts).filter(|(file, _)| file.nested.is_none()).filter_map(|(_, facts)| facts.as_ref())
    {
        external.imported.extend(facts.imported_names().map(str::to_owned));
        external.read.extend(facts.read_names().map(str::to_owned));
        external.assigned.extend(facts.assigned_names().map(str::to_owned));
        external.opaque.extend(facts.doctests().map(str::to_owned));
        external.computed.extend(facts.computed_reads());
    }
    external
}

// The passes of a run's files, which are `None` when there is no Python file to check, and the paths the run names that
// do not exist, with the message each reports.
type Walked<T> = (Option<Vec<Vec<T>>>, Vec<(PathBuf, String)>);

// The inputs the stages of a run pass on unchanged, so each stage takes them as one argument.
pub(super) struct Plumbing<'a, T> {
    pub(super) cache_dir: Option<&'a Path>,
    pub(super) rewrote: Option<&'a Rewrite<'a, T>>,
    pub(super) reuse: Option<&'a Reuse<'a, T>>,
    pub(super) ending: Ending,
    pub(super) overlay: &'a Overlay,
    pub(super) cwd: &'a Path,
}

impl<'a, T> Plumbing<'a, T> {
    pub(super) fn plain(ending: Ending, overlay: &'a Overlay, cwd: &'a Path) -> Self {
        Self { cache_dir: None, rewrote: None, reuse: None, ending, overlay, cwd }
    }
}

// A check of a file's cached reports, given the file, its cache and its key, from which the run skips the index.
type Reuse<'a, T> = dyn Fn(&RunFile, &Cache, FileKey) -> Option<T> + Sync + 'a;

// Whether a pass rewrote a file, which a later round has to index again.
type Rewrite<'a, T> = dyn Fn(&T) -> bool + Sync + 'a;
