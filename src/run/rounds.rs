use std::hash::Hasher;
use std::path::Path;
use std::time::Instant;

use anyhow::Result;
use log::debug;
use rayon::prelude::*;

use super::Overlay;
use super::check::Plumbing;
use super::files::{ProjectInputs, RunFile};
use crate::lint::Linter;
use crate::project::cache::FileKey;
use crate::project::{DocReferences, External, FileFacts, ImportResolver, IndexInputs, Parts, Project, SearchRoots};
use crate::run::cache::Cache;
use crate::syntax::identifier_words;

// One discovered Python file, with its linter, and its package's cache with its key when the cache is on.
pub(super) struct DiscoveredFile<'a> {
    pub(super) path: &'a Path,
    pub(super) linter: Linter<'a>,
    pub(super) cache: Option<(&'a Cache<'a>, FileKey)>,
}

// The import index of one project's `files`, built from the facts of each one that has them, and then its first `linted`
// files linted, each with its passes. The linted files come first. `parts` are the facts the rules read, and `docs` is
// what the documentation of the linted files' projects names.
#[expect(clippy::too_many_arguments, reason = "the project's inputs, which each stage passes on")]
pub(super) fn index_and_lint<'f, T: Send>(
    files: &'f [RunFile<'f>],
    (parts, project, startup): (Parts, &ProjectInputs, &[String]),
    roots: &SearchRoots,
    mut facts: Vec<Option<FileFacts>>,
    docs: &DocReferences,
    external: &External,
    file_caches: &[Option<(&'f Cache, FileKey)>],
    plumbing: &Plumbing<'_, T>,
    work: &(dyn Fn(&DiscoveredFile) -> T + Sync),
) -> Result<Vec<Vec<T>>> {
    let start = Instant::now();
    let linted: Vec<&RunFile> = files.iter().filter(|file| file.linted).collect();
    let inputs = IndexInputs {
        entry_points: &project.entry_points,
        build_hooks: &project.build_hooks,
        coverage_plugins: &project.coverage_plugins,
        startup,
        roles: &project.roles,
        test_modules: &project.test_modules,
        distribution_roots: &project.distribution_roots,
        docs,
        duplicate_statements: project.duplicate_statements,
        probed: project.probed,
        external,
    };
    let mut imports = ImportResolver::new(
        roots,
        project.entry_points.iter().map(|(root, _)| root.as_path()),
        project.coverage_paths.iter().map(|(root, directory)| (root.as_path(), directory.as_path())),
    )?;
    let paths: Vec<&Path> = files.iter().map(|file| file.absolute.as_path()).collect();
    let indexed = index_project(&paths, &facts, &mut imports, &inputs);
    debug!("Indexed {} files in: {:?}", facts.iter().flatten().count(), start.elapsed());
    let start = Instant::now();
    let linted_files = &files[..linted.len()];
    let index = Index {
        facts: &mut facts,
        paths: &paths,
        resolver: &mut imports,
        inputs: &inputs,
        parts,
        overlay: plumbing.overlay,
    };
    let passes = lint_rounds(linted_files, index, indexed, file_caches, plumbing, work);
    debug!("Checked {:?} files in: {:?}", passes.len(), start.elapsed());
    plumbing.ending.end(facts);
    plumbing.ending.end(imports);
    Ok(passes)
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
    // The fixes of a diff run, whose sources the rounds read in place of the disk's.
    overlay: &'i Overlay,
}

// Each file's passes, in order. A fix in one file can change what another learns from the index, as folding
// `mock.uuid = 7` into the mock's constructor leaves no assignment to stop a dataclass with a `uuid` field from
// freezing. So with `rewrote` set, the files a round rewrites have their facts collected again, and each file whose
// answers from the index change is linted again, until a round rewrites nothing. A rewritten file's cycle and repeated
// statements are ranges of the source it had before, so a rewritten file that reports one is linted again even when
// its answers did not change.
fn lint_rounds<'f, T: Send>(
    files: &'f [RunFile<'f>],
    mut index: Index<'_, 'f>,
    mut project: Project,
    file_caches: &[Option<(&'f Cache, FileKey)>],
    plumbing: &Plumbing<'_, T>,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Vec<Vec<T>> {
    // Two files whose fixes keep changing each other's facts would not settle; past this many rounds, the last
    // reports stand.
    const ROUNDS: usize = 8;
    // A file an applied fix rewrote has a new key. A diff run's fixes leave the disk as it was, so every file a later
    // round lints sees an index the disk does not hold, and nothing such a round reports is cached.
    let overlay = index.overlay;
    let recache = |at: usize| {
        if !overlay.is_empty() {
            return None;
        }
        Some((file_caches[at]?.0, FileKey::of(files[at].path)?))
    };
    // Without rewrites nothing indexes again, so the facts and resolved module names go before the files are linted.
    if plumbing.rewrote.is_none() {
        plumbing.ending.end(std::mem::take(index.facts));
        index.resolver.release(plumbing.ending);
    }
    let lint = |file: &RunFile, project: &Project, cache| {
        work(&DiscoveredFile { path: file.path, linter: file_linter(file, project), cache })
    };
    let mut passes: Vec<Vec<T>> =
        files.par_iter().zip(file_caches).map(|(file, cache)| vec![lint(file, &project, *cache)]).collect();
    let Some(rewrote) = plumbing.rewrote else {
        plumbing.ending.end(project);
        return passes;
    };
    let mut rewritten: Vec<usize> = (0..passes.len()).filter(|&at| passes[at].iter().any(rewrote)).collect();
    // Each file's words, read when a round first needs them and again once a round rewrites the file.
    let mut words: Vec<Option<Words>> = vec![None; files.len()];
    let parts = index.parts;
    for _ in 1..ROUNDS {
        if rewritten.is_empty() {
            break;
        }
        for &at in &rewritten {
            words[at] = None;
        }
        let collect = |at: usize| file_facts(&files[at], recache(at), parts, overlay);
        let (next, dependents) = reindex(&mut index, &rewritten, collect, files, &mut words, &project);
        project = next;
        let relinted: Vec<(usize, T)> =
            dependents.into_par_iter().map(|at| (at, lint(&files[at], &project, recache(at)))).collect();
        rewritten = relinted.iter().filter(|(_, pass)| rewrote(pass)).map(|(at, _)| *at).collect();
        for (at, pass) in relinted {
            passes[at].push(pass);
        }
    }
    plumbing.ending.end(project);
    passes
}

// A file's import facts: those its cache holds while the file is unchanged, and otherwise freshly collected and stored
// there. Cached facts without a part the run's rules read, or resolved against other module paths, as when the search
// roots changed, are collected again, keeping the parts they had, so runs with rules that read different parts, as a
// hook and a full run, do not each collect the file again.
pub(super) fn file_facts(
    file: &RunFile,
    cache: Option<(&Cache<'_>, FileKey)>,
    parts: Parts,
    overlay: &Overlay,
) -> Option<FileFacts> {
    let (module_path, importable) = (file.module_path.as_deref(), file.importable.as_deref());
    let cached = cache.and_then(|(cache, key)| cache.facts(file.path, key));
    let parts = match cached {
        Some(facts) if facts.covers(parts, module_path, importable) => return Some(facts),
        Some(facts) => parts.with(facts.parts()),
        None => parts,
    };
    let source = overlay.source(file.path).ok()?;
    let facts = FileFacts::collect(file.path, &source, module_path, importable, parts)?;
    if let Some((cache, key)) = cache {
        cache.set_facts(file.path, key, &facts);
    }
    Some(facts)
}

// The index once the `rewritten` files have their facts collected and their module names resolved again, and the
// positions of the files whose answers from it differ from those of `project`, or that a rewrite left with a cycle or
// repeated statements to report. Each file's `words` are read the first time a changed name needs them.
fn reindex(
    index: &mut Index<'_, '_>,
    rewritten: &[usize],
    collect: impl Fn(usize) -> Option<FileFacts> + Sync,
    files: &[RunFile],
    words: &mut [Option<Words>],
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
        let overlay = index.overlay;
        words.par_iter_mut().zip(files).filter(|(known, _)| known.is_none()).for_each(|(known, file)| {
            *known = Some(Words::of(file.path, overlay));
        });
    }
    let dependents = (0..files.len())
        .into_par_iter()
        .filter(|&at| {
            let path = files[at].absolute.as_path();
            let answers = next.file(path);
            let mentions = || words[at].as_ref().is_some_and(|held| changed.iter().any(|name| held.holds(name)));
            let reports = answers.import_cycle().is_some() || !answers.duplicate_runs().is_empty();
            answers != project.file(path) || mentions() || (reports && rewritten.contains(&at))
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
// answers. Each run keeps these for every file through its rounds, so it holds their hashes, sorted: a hash two words
// share only lints a file again that did not need it.
#[derive(Clone)]
struct Words(Vec<u64>);

impl Words {
    fn of(path: &Path, overlay: &Overlay) -> Self {
        let source = overlay.source(path).unwrap_or_default();
        let mut hashes: Vec<u64> = identifier_words(&source).map(word_hash).collect();
        hashes.sort_unstable();
        hashes.dedup();
        Self(hashes)
    }

    fn holds(&self, word: &str) -> bool {
        self.0.binary_search(&word_hash(word)).is_ok()
    }
}

fn word_hash(word: &str) -> u64 {
    let mut hasher = rustc_hash::FxHasher::default();
    hasher.write(word.as_bytes());
    hasher.finish()
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
