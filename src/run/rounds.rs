use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::Result;
use itertools::Itertools;
use log::{debug, error};
use rayon::prelude::*;
use std::hash::Hasher;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::package::PackageRoot;
use ruff_linter::packaging::detect_package_root;
use ruff_linter::{SuppressionKind, warn_user_once};
use ruff_python_ast::helpers::to_module_path;
use rustc_hash::{FxHashMap, FxHashSet};

use super::conflicts::{self, Scope};
use super::discover::{Discovered, discover, linted_roots};
use super::{Diagnostics, Mode, Resolved, StdinLint, lint_path, lint_stdin, write_source};
use crate::cli::printer::FullDisplay;
use crate::lint::Linter;
use crate::lint::registry::{Rule, RuleSet};
use crate::project::cache::{Cache, Caches, FileKey, Fingerprints};
use crate::project::{DocReferences, Ending, FileFacts, ImportResolver, IndexInputs, Parts, Project, SearchRoots};
use crate::rules::classes::dataclass_kw_only::suppresses_rewrite;
use crate::settings::{FileSettings, ModuleRole, Settings};

// Ruff's `add_noqa`: a file that cannot be read or written is logged and left as it is.
pub(crate) fn add_suppressions(
    files: &[PathBuf],
    resolved: &Resolved,
    kind: SuppressionKind,
    reason: Option<&str>,
) -> Result<usize> {
    // A suppression comment changes no file's facts, so one round is final.
    let roots = linted_roots(files);
    let counts =
        for_each_file(files, resolved, None, None, None, Ending::Drop, |DiscoveredFile { path, linter, .. }| {
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
        })?;
    Ok(counts.into_iter().flatten().flatten().sum())
}

// Lints the files of a run, reusing their cached reports when `use_cache` is set. With `full` set, each file's cached
// reports also keep the text the `full` output shows them as.
pub(crate) fn check_files(
    files: &[PathBuf],
    resolved: &Resolved,
    mode: &Mode,
    use_cache: bool,
    full: Option<&FullDisplay>,
    ending: Ending,
) -> Result<Diagnostics> {
    let cache_dir = use_cache.then_some(resolved.cache_dir.as_path());
    let fingerprints = Fingerprints::default();
    // Only applied fixes reach the disk, where other files' facts come from.
    let rewrote = |diagnostics: &Diagnostics| !diagnostics.fixed.is_empty();
    // Without fixes a file's reports need no index once the run's digest matches the one they were stored under.
    let reuse = |file: &RunFile, cache: &Cache, key: FileKey| {
        let base = fingerprints.base(&file.settings, &file.ruff.linter, file.module_path.as_ref(), file.sibling_init);
        cache.reused(file.path, key, base, full)
    };
    let reuse =
        mode.fix.is_generate().then_some(&reuse as &(dyn Fn(&RunFile, &Cache, FileKey) -> Option<Diagnostics> + Sync));
    let results =
        for_each_file(files, resolved, cache_dir, mode.fix.is_apply().then_some(&rewrote), reuse, ending, |file| {
            let lint = |settled| lint_path(file.path, &file.linter, mode, settled);
            let Some((cache, key)) = file.cache else {
                return lint(None).diagnostics;
            };
            cache.lint_or_reuse(file.path, key, fingerprints.of(&file.linter, mode), full, lint)
        })?;
    // A later pass over a file replaces its reports, while the fixes of every pass count.
    let file_results = results.into_iter().flatten().filter_map(|passes| {
        passes.into_iter().reduce(|mut earlier, later| {
            earlier.inner = later.inner;
            earlier.fixed += later.fixed;
            earlier
        })
    });
    Ok(file_results.fold(Diagnostics::default(), |left, right| left + right))
}

// Ruff's `check_stdin`: the source comes from standard input, and nothing else is indexed. Otherwise it is linted as
// the file it stands for would be, so its settings, package and search roots come from that file's absolute path.
pub(crate) fn check_stdin(path: &Path, source: &str, resolved: &Resolved, mode: &Mode) -> Result<StdinLint> {
    let ruff = &resolved.pyproject_config.settings;
    let absolute = std::path::absolute(path)?;
    let settings = resolved.settings_resolver.for_file(&absolute)?;
    let package = absolute
        .parent()
        .and_then(|parent| detect_package_root(parent, &ruff.linter.namespace_packages))
        .map(PackageRoot::root);
    let roots = SearchRoots::new(ruff, package);
    let files = [RunFile::new(&absolute, &settings, ruff, resolved.pyproject_config.path.as_deref(), package, &roots)];
    ensure_compatible(&files, resolved)?;
    let rules = rules_of(&files);
    let index = index_parts(&files, 1, &roots);
    let read = (vec![None], documentation(&files, rules, None));
    let linted = (1, wanted_parts(rules), index);
    let mut passes = index_and_lint(&files, linted, &roots, read, &[None], None, Ending::Drop, |file| {
        lint_stdin(path, source, &file.linter, mode)
    })?;
    Ok(passes.pop().and_then(|mut file| file.pop()).expect("the standard input is linted once"))
}

// Ruff's per-file loop over the discovered files, with the import index of the whole run built first, and the caches
// in `cache_dir` read before and written after. With `rewrote` set, the files a round rewrites are indexed and linted
// again, as `lint_rounds` tells. `None` when there is no Python file to check.
fn for_each_file<T: Send>(
    files: &[PathBuf],
    resolved: &Resolved,
    cache_dir: Option<&Path>,
    rewrote: Option<&(dyn Fn(&T) -> bool + Sync)>,
    reuse: Option<&Reuse<'_, T>>,
    ending: Ending,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Option<Vec<Vec<T>>>> {
    let start = Instant::now();
    let Discovered { python, unlinted, resolver } = discover(files, resolved)?;
    debug!("Identified files to lint in: {:?}", start.elapsed());
    if python.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
        return Ok(None);
    }
    let paths: Vec<&Path> = python.iter().chain(&unlinted).map(PathBuf::as_path).collect();
    let settings: Vec<Arc<Settings>> =
        paths.iter().map(|path| resolved.settings_resolver.for_file(path)).collect::<Result<_>>()?;
    let packages = resolver.package_roots(&paths);
    let roots = SearchRoots::new(&resolved.pyproject_config.settings, packages.values().flatten().copied());
    let indexed: Vec<RunFile> = paths
        .iter()
        .zip(&settings)
        .map(|(&path, settings)| {
            let (ruff, ruff_config) = resolver.resolve_with_path(path);
            let package = path.parent().and_then(|parent| packages.get(parent)).copied().flatten();
            RunFile::new(path, settings, ruff, ruff_config, package, &roots)
        })
        .collect();
    ensure_compatible(&indexed[..python.len()], resolved)?;
    let rules = rules_of(&indexed[..python.len()]);
    let parts = wanted_parts(rules);
    if let Some(cache_dir) = cache_dir {
        Caches::prepare(cache_dir);
    }
    // The documentation walk reads files of its own, so it runs while the caches open and the facts are collected,
    // which on a warm run is most of what comes before the index.
    std::thread::scope(|scope| {
        let docs = scope.spawn(|| documentation(&indexed[..python.len()], rules, cache_dir));
        let joined = || docs.join().unwrap_or_else(|panic| std::panic::resume_unwind(panic));
        index_cached(
            &indexed,
            (python.len(), parts),
            (&roots, &packages),
            cache_dir,
            joined,
            (rewrote, reuse),
            ending,
            work,
        )
    })
}

// A check of a file's cached reports, given the file, its cache and its key, from which the run skips the index.
type Reuse<'a, T> = dyn Fn(&RunFile, &Cache, FileKey) -> Option<T> + Sync + 'a;

// Whether a pass rewrote a file, which a later round has to index again.
type Rewrite<'a, T> = dyn Fn(&T) -> bool + Sync + 'a;

// The caches of the run opened, then either every linted file answered from them under the run's digest, or the facts of
// the files read from them or collected, and the files indexed and linted. `docs` gives what the documentation names
// once the walk that runs alongside is done.
#[expect(clippy::too_many_arguments, reason = "the run's inputs, which each stage passes on")]
fn index_cached<'f, T: Send>(
    indexed: &'f [RunFile<'f>],
    (linted, parts): (usize, Parts),
    (roots, packages): (&SearchRoots, &FxHashMap<&'f Path, Option<PackageRoot<'f>>>),
    cache_dir: Option<&Path>,
    docs: impl FnOnce() -> DocReferences,
    (rewrote, reuse): (Option<&Rewrite<'_, T>>, Option<&Reuse<'_, T>>),
    ending: Ending,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Option<Vec<Vec<T>>>> {
    let caches = cache_dir.map(|cache_dir| {
        Caches::open(cache_dir, packages.iter().map(|(parent, package)| package.map_or(*parent, PackageRoot::path)))
    });
    let docs = docs();
    let index = index_parts(indexed, linted, roots);
    // Only the cache reads the digest, which stats every file and walks every directory an import can name, so a run
    // without one skips it.
    let opened = caches.as_ref().map(|caches| {
        let keys: Vec<Option<FileKey>> = indexed.par_iter().map(|file| FileKey::of(file.path)).collect();
        let run = run_digest(indexed, &keys, linted, roots, &index, &docs, parts);
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
                    let root = file.package.map(PackageRoot::path).or_else(|| file.path.parent())?;
                    Some((views.get(root)?, (*key)?))
                })
                .collect()
        },
    );
    if let Some(reuse) = reuse {
        let reused: Option<Vec<Vec<T>>> = indexed[..linted]
            .par_iter()
            .zip(&file_caches[..linted])
            .map(|(file, cached)| {
                let (cache, key) = (*cached)?;
                Some(vec![reuse(file, cache, key)?])
            })
            .collect();
        if let Some(passes) = reused {
            persist_views(opened.map(|(views, _)| views), ending)?;
            return Ok(Some(passes));
        }
    }
    let facts: Vec<Option<FileFacts>> =
        indexed.par_iter().zip(&file_caches).map(|(file, cached)| file_facts(file, *cached, parts)).collect();
    let passes =
        index_and_lint(indexed, (linted, parts, index), roots, (facts, docs), &file_caches, rewrote, ending, work)?;
    persist_views(opened.map(|(views, _)| views), ending)?;
    Ok(Some(passes))
}

// Writes the caches the run changed, each package's on its own thread.
fn persist_views(views: Option<FxHashMap<&Path, Cache>>, ending: Ending) -> Result<()> {
    let Some(views) = views else {
        return Ok(());
    };
    views.into_values().collect::<Vec<_>>().into_par_iter().try_for_each(|cache| cache.persist(ending))
}

// The inputs of the import index that the linted files and the run's files determine: the entry points, the files
// whose calls a fix rewrites, where each file sits in its library's API, the test modules and the duplicate window.
struct IndexParts<'f> {
    entry_points: Vec<String>,
    rewrites_calls: FxHashSet<&'f Path>,
    roles: FxHashMap<&'f Path, ModuleRole>,
    test_modules: FxHashSet<&'f Path>,
    duplicate_statements: Option<usize>,
}

fn index_parts<'f>(files: &'f [RunFile<'f>], linted: usize, roots: &SearchRoots) -> IndexParts<'f> {
    let linted_files = &files[..linted];
    let entry_points: Vec<String> = linted_files
        .iter()
        .map(|file| file.settings.project)
        .unique_by(|settings| std::ptr::from_ref(*settings))
        .flat_map(|settings| settings.metadata.entry_points.iter().cloned())
        .sorted_unstable()
        .dedup()
        .collect();
    IndexParts {
        entry_points,
        rewrites_calls: linted_files
            .iter()
            .filter(|file| file.settings.rewrites_dataclass_calls())
            .map(|file| file.absolute.as_path())
            .collect(),
        roles: module_roles(files, roots),
        test_modules: files
            .iter()
            .filter(|file| file.settings.is_test_module(file.path))
            .map(|file| file.absolute.as_path())
            .collect(),
        duplicate_statements: duplicate_window(linted_files, rules_of(linted_files)),
    }
}

// The digest of what an import index of a run is built from: the working directory, the search roots and the layout of
// the directories its resolver reads, every run file with its key and where it sits, the index's own inputs, the
// documentation and the facts the rules read. A cache entry stored under the same digest was computed from the same
// index, which then need not be built.
fn run_digest(
    files: &[RunFile],
    keys: &[Option<FileKey>],
    linted: usize,
    roots: &SearchRoots,
    index: &IndexParts,
    docs: &DocReferences,
    parts: Parts,
) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    std::env::current_dir().unwrap_or_default().cache_key(&mut hasher);
    roots.cache_key(&mut hasher);
    roots.layout_key(files.iter().map(|file| file.path), &mut hasher);
    linted.cache_key(&mut hasher);
    for (file, key) in files.iter().zip(keys) {
        file.path.cache_key(&mut hasher);
        key.cache_key(&mut hasher);
        file.package.map(PackageRoot::path).map(Path::to_path_buf).cache_key(&mut hasher);
        file.module_path.cache_key(&mut hasher);
        file.importable.cache_key(&mut hasher);
    }
    index.entry_points.cache_key(&mut hasher);
    index.duplicate_statements.cache_key(&mut hasher);
    for set in [&index.rewrites_calls, &index.test_modules] {
        let mut paths: Vec<&Path> = set.iter().copied().collect();
        paths.sort_unstable();
        paths.cache_key(&mut hasher);
    }
    let mut roles: Vec<(&Path, ModuleRole)> = index.roles.iter().map(|(path, role)| (*path, *role)).collect();
    roles.sort_unstable_by_key(|(path, _)| *path);
    for (path, role) in roles {
        path.cache_key(&mut hasher);
        role.cache_key(&mut hasher);
    }
    docs.cache_key(&mut hasher);
    parts.cache_key(&mut hasher);
    hasher.finish()
}

// A file the run indexes, with what indexing and linting it take, worked out once: its settings and Ruff's with the
// configuration file they come from, its package, and the module it imports as from that package. Ruff sees no package
// around a file of a namespace package, yet Python resolves its relative imports, so such a file is `importable` from
// a search root instead.
struct RunFile<'a> {
    path: &'a Path,
    // The path the index keys the file by. The run's paths may be relative to the working directory, so they are made
    // absolute once here rather than at every lookup.
    absolute: PathBuf,
    settings: FileSettings<'a>,
    ruff: &'a ruff_workspace::Settings,
    ruff_config: Option<&'a Path>,
    package: Option<PackageRoot<'a>>,
    module_path: Option<Vec<String>>,
    importable: Option<Vec<String>>,
    sibling_init: bool,
}

impl<'a> RunFile<'a> {
    fn new(
        path: &'a Path,
        settings: &'a Settings,
        ruff: &'a ruff_workspace::Settings,
        ruff_config: Option<&'a Path>,
        package: Option<PackageRoot<'a>>,
        roots: &SearchRoots,
    ) -> Self {
        let module_path = package.and_then(|package| to_module_path(package.path(), path));
        let importable = if module_path.is_none() { roots.module_path(path) } else { None };
        let settings = FileSettings::new(settings, ruff, path);
        // A namespace package has no `__init__.py`, which a fingerprint and a rule read only when a rule asks.
        let sibling_init =
            settings.rules.overlaps(RuleSet::sibling_init()) && path.with_file_name("__init__.py").is_file();
        let absolute = std::path::absolute(path).unwrap_or_else(|_| path.to_path_buf());
        Self { path, absolute, settings, ruff, ruff_config, package, module_path, importable, sibling_init }
    }
}

// One discovered Python file, with its linter, and its package's cache with its key when the cache is on.
struct DiscoveredFile<'a> {
    path: &'a Path,
    linter: Linter<'a>,
    cache: Option<(&'a Cache<'a>, FileKey)>,
}

// The import index of the run's `files`, built from the facts of each one that has them, and then their first
// `linted` files linted, each with its passes. `enabled` is the union of the linted files' rules, and `parts` the
// facts they read; `docs` is what the documentation of the linted files' projects names.
#[expect(clippy::too_many_arguments, reason = "the run's inputs, which each stage passes on")]
fn index_and_lint<'f, T: Send>(
    files: &'f [RunFile<'f>],
    (linted, parts, index): (usize, Parts, IndexParts<'f>),
    roots: &SearchRoots,
    (mut facts, docs): (Vec<Option<FileFacts>>, DocReferences),
    file_caches: &[Option<(&'f Cache, FileKey)>],
    rewrote: Option<&(dyn Fn(&T) -> bool + Sync)>,
    ending: Ending,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Vec<Vec<T>>> {
    let start = Instant::now();
    let linted_files = &files[..linted];
    let inputs = IndexInputs {
        entry_points: &index.entry_points,
        rewrites_calls: &index.rewrites_calls,
        roles: &index.roles,
        test_modules: &index.test_modules,
        docs: &docs,
        duplicate_statements: index.duplicate_statements,
    };
    let mut imports = ImportResolver::new(roots)?;
    let paths: Vec<&Path> = files.iter().map(|file| file.absolute.as_path()).collect();
    let project = index_project(&paths, &facts, &mut imports, &inputs);
    debug!("Indexed {} files in: {:?}", facts.iter().flatten().count(), start.elapsed());
    let start = Instant::now();
    let index = Index { facts: &mut facts, paths: &paths, resolver: &mut imports, inputs: &inputs, parts };
    let passes = lint_rounds(linted_files, index, project, file_caches, rewrote, ending, work);
    debug!("Checked {:?} files in: {:?}", passes.len(), start.elapsed());
    ending.end(facts);
    ending.end(imports);
    Ok(passes)
}

// The rules the run's files turn on, one union for every per-run check.
fn rules_of(files: &[RunFile]) -> RuleSet {
    files.iter().fold(RuleSet::default(), |rules, file| rules.union(file.settings.rules))
}

// The facts the run's rules read, which only those rules' runs need collected.
fn wanted_parts(rules: RuleSet) -> Parts {
    rules.iter().fold(Parts::default(), |parts, rule| parts.with(rule.fact_parts()))
}

// The fewest statements a duplicate block spans: each file reports the blocks its own setting allows, so the index finds
// the shortest any file asks for. `None` when no file checks for duplicate code, whose statements the facts then lack.
fn duplicate_window(files: &[RunFile], rules: RuleSet) -> Option<usize> {
    if !rules.contains(Rule::DuplicateCode) {
        return None;
    }
    files.iter().map(|file| file.settings.project.knobs.min_duplicate_statements).min()
}

// What the documentation of each project with a `pyproject.toml` names, read only when a rule that asks is on.
fn documentation(files: &[RunFile], rules: RuleSet, cache_dir: Option<&Path>) -> DocReferences {
    if !rules.overlaps(RuleSet::documentation()) {
        return DocReferences::default();
    }
    let roots = files
        .iter()
        .filter(|file| file.settings.project.path.is_some())
        .map(|file| (file.settings.project.root.as_path(), file.ruff))
        .unique_by(|(root, _)| *root);
    DocReferences::collect(roots, cache_dir)
}

fn ensure_compatible(files: &[RunFile], resolved: &Resolved) -> Result<()> {
    let scopes = files
        .iter()
        .map(|file| Scope {
            path: file.path,
            settings: file.settings.clone(),
            ruff: file.ruff,
            ruff_config: file.ruff_config,
            package: file.package,
        })
        .collect_vec();
    conflicts::ensure_compatible(&scopes, resolved.names_rules())
}

// What a fix round indexes the run from: each file's facts, where their module names lead, and what else the index
// reads.
struct Index<'i, 'f> {
    // Each file's facts by its position in the run, which is also its position in `paths`.
    facts: &'i mut Vec<Option<FileFacts>>,
    paths: &'i [&'f Path],
    resolver: &'i mut ImportResolver,
    inputs: &'i IndexInputs<'i>,
    parts: Parts,
}

// Each file's passes, in order. A fix in one file can change what another learns from the index, as folding
// `mock.uuid = 7` into the mock's constructor leaves no assignment to stop a dataclass with a `uuid` field from
// freezing. So with `rewrote` set, the files a round rewrites have their facts collected again, and each file whose
// answers from the index change is linted again, until a round rewrites nothing.
fn lint_rounds<'f, T: Send>(
    files: &'f [RunFile<'f>],
    mut index: Index<'_, 'f>,
    mut project: Project,
    file_caches: &[Option<(&'f Cache, FileKey)>],
    rewrote: Option<&(dyn Fn(&T) -> bool + Sync)>,
    ending: Ending,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Vec<Vec<T>> {
    // Two files whose fixes keep changing each other's facts would not settle; past this many rounds, the last
    // reports stand.
    const ROUNDS: usize = 8;
    // A file a fix rewrote has a new key.
    let recache = |at: usize| Some((file_caches[at]?.0, FileKey::of(files[at].path)?));
    // Without rewrites nothing indexes again, so the facts and resolved module names go before the files are linted.
    if rewrote.is_none() {
        ending.end(std::mem::take(index.facts));
        index.resolver.release(ending);
    }
    let lint = |file: &RunFile, project: &Project, cache| {
        work(&DiscoveredFile { path: file.path, linter: file_linter(file, project), cache })
    };
    let mut passes: Vec<Vec<T>> =
        files.par_iter().zip(file_caches).map(|(file, cache)| vec![lint(file, &project, *cache)]).collect();
    let Some(rewrote) = rewrote else {
        ending.end(project);
        return passes;
    };
    let mut rewritten: Vec<usize> = (0..passes.len()).filter(|&at| passes[at].iter().any(rewrote)).collect();
    // Each file's words, read when a round first needs them and again once a round rewrites the file.
    let mut words: Vec<Option<FxHashSet<String>>> = vec![None; files.len()];
    let parts = index.parts;
    for _ in 1..ROUNDS {
        if rewritten.is_empty() {
            break;
        }
        for &at in &rewritten {
            words[at] = None;
        }
        let collect = |at: usize| file_facts(&files[at], recache(at), parts);
        let (next, dependents) = reindex(&mut index, &rewritten, collect, files, &mut words, &project);
        project = next;
        let relinted: Vec<(usize, T)> =
            dependents.into_par_iter().map(|at| (at, lint(&files[at], &project, recache(at)))).collect();
        rewritten = relinted.iter().filter(|(_, pass)| rewrote(pass)).map(|(at, _)| *at).collect();
        for (at, pass) in relinted {
            passes[at].push(pass);
        }
    }
    ending.end(project);
    passes
}

// Where each file sits in its library's public API, by the first of the names the file imports as, from its package or
// from a search root, that an API package holds. Unlinted files count too: an `__init__.py` Ruff's `lint.exclude` leaves
// out still re-exports public names.
fn module_roles<'s>(files: &'s [RunFile<'_>], roots: &SearchRoots) -> FxHashMap<&'s Path, ModuleRole> {
    files
        .par_iter()
        .map(|file| {
            let settings = file.settings.project;
            let module_paths = file.module_path.iter().cloned().chain(roots.module_paths(file.path)).collect_vec();
            let roles: Vec<ModuleRole> =
                module_paths.iter().map(|module_path| settings.role(file.path, Some(module_path))).collect();
            let role = roles.iter().copied().find(|role| role.public).or_else(|| roles.first().copied());
            (file.absolute.as_path(), role.unwrap_or_else(|| settings.role(file.path, None)))
        })
        .collect()
}

// A file's import facts: those its cache holds while the file is unchanged, and otherwise freshly collected and stored
// there. Cached facts without a part the run's rules read are collected again.
fn file_facts(file: &RunFile, cache: Option<(&Cache<'_>, FileKey)>, parts: Parts) -> Option<FileFacts> {
    let cached = cache.and_then(|(cache, key)| cache.facts(file.path, key));
    if let Some(facts) = cached.filter(|facts| facts.covers(parts)) {
        return Some(facts);
    }
    let facts = FileFacts::collect(
        file.path,
        file.module_path.as_deref(),
        file.importable.as_deref(),
        &suppresses_rewrite,
        parts,
    )?;
    if let Some((cache, key)) = cache {
        cache.set_facts(file.path, key, facts.clone());
    }
    Some(facts)
}

// The index once the `rewritten` files have their facts collected and their module names resolved again, and the
// positions of the files whose answers from it differ from those of `project`. Each file's `words` are read the first
// time a changed name needs them.
fn reindex(
    index: &mut Index<'_, '_>,
    rewritten: &[usize],
    collect: impl Fn(usize) -> Option<FileFacts> + Sync,
    files: &[RunFile],
    words: &mut [Option<FxHashSet<String>>],
    project: &Project,
) -> (Project, Vec<usize>) {
    // A fix never leaves a file that does not parse, so a rewritten file keeps having facts.
    let fresh: Vec<(usize, FileFacts)> = rewritten.par_iter().filter_map(|&at| Some((at, collect(at)?))).collect();
    for &at in rewritten {
        index.resolver.forget(files[at].absolute.as_path());
    }
    for (at, facts) in fresh {
        index.facts[at] = Some(facts);
    }
    let next = index_project(index.paths, index.facts, index.resolver, index.inputs);
    let changed = next.changed_names(project);
    if !changed.is_empty() {
        words.par_iter_mut().zip(files).filter(|(known, _)| known.is_none()).for_each(|(known, file)| {
            *known = Some(words_of(file.path));
        });
    }
    let dependents = (0..files.len())
        .into_par_iter()
        .filter(|&at| {
            let path = files[at].absolute.as_path();
            let mentions = || words[at].as_ref().is_some_and(|held| changed.iter().any(|name| held.contains(*name)));
            next.file(path) != project.file(path) || mentions()
        })
        .collect();
    (next, dependents)
}

// The index of the files that have facts, which are given by absolute path.
fn index_project(
    paths: &[&Path],
    facts: &[Option<FileFacts>],
    resolver: &mut ImportResolver,
    inputs: &IndexInputs,
) -> Project {
    let indexed: Vec<(&Path, &FileFacts)> =
        paths.iter().zip(facts).filter_map(|(path, facts)| Some((*path, facts.as_ref()?))).collect();
    Project::index(&indexed, resolver, inputs)
}

// The names a file holds, to tell whether it names any of the attribute or base class names whose answers changed; the
// rules ask about names the file itself holds, such as its dataclass fields, so a file without one gets the same
// answers.
fn words_of(path: &Path) -> FxHashSet<String> {
    let source = std::fs::read_to_string(path).unwrap_or_default();
    source.split(|char: char| !(char.is_alphanumeric() || char == '_')).map(str::to_owned).collect()
}

// A file's linter, from simplint's settings and Ruff's for it and the package it sits in.
fn file_linter<'a>(file: &RunFile<'a>, project: &'a Project) -> Linter<'a> {
    Linter::new(
        file.ruff,
        file.settings.clone(),
        project.file(file.absolute.as_path()),
        file.package,
        file.module_path.clone(),
        file.sibling_init,
    )
}
