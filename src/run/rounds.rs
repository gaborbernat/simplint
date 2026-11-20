use std::hash::Hasher;
use std::path::Path;
use std::sync::Mutex;
use std::time::Instant;

use anyhow::Result;
use log::debug;
use rayon::prelude::*;

use super::Overlay;
use super::check::Plumbing;
use super::files::{ProjectInputs, RunFile, WorkerParse};
use crate::lint::{Linter, parse_source};
use crate::project::cache::FileKey;
use crate::project::{DocReferences, External, FileFacts, ImportResolver, IndexInputs, Parts, Project, SearchRoots};
use crate::run::cache::Cache;
use crate::syntax::identifier_words;

pub(super) struct DiscoveredFile<'a> {
    pub(super) path: &'a Path,
    pub(super) linter: Linter<'a>,
    pub(super) cache: Option<(&'a Cache<'a>, FileKey)>,
}

// The linted files come first in `files`.
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
    let inputs = IndexInputs {
        entry_points: &project.entry_points,
        build_hooks: &project.build_hooks,
        coverage_plugins: &project.coverage_plugins,
        startup,
        roles: &project.roles,
        test_modules: &project.test_modules,
        distribution_roots: &project.distribution_roots,
        dependency_hosts: &project.dependency_hosts,
        docs,
        duplicate_statements: project.duplicate_statements,
        probed: project.probed,
        external,
    };
    let mut imports = ImportResolver::new(
        plumbing.cwd,
        roots,
        project.entry_points.iter().map(|(root, _)| root.as_path()),
        project.coverage_paths.iter().map(|(root, directory)| (root.as_path(), directory.as_path())),
    )?;
    let paths: Vec<&Path> = files.iter().map(|file| file.absolute.as_path()).collect();
    let indexed = index_project(&paths, &facts, &mut imports, &inputs);
    debug!("Indexed {} files in: {:?}", facts.iter().flatten().count(), start.elapsed());
    let start = Instant::now();
    let linted_files = &files[..files.iter().filter(|file| file.linted).count()];
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
// answers from the index change is linted again, until a round rewrites nothing.
//
// A rewritten file's cycle and repeated statements are ranges of the source it had before, so a rewritten file that
// reports one is linted again even when its answers did not change.
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
        Some((file_caches[at]?.0, FileKey::of(files[at].path, filetime::FileTime::now())?))
    };
    // Without rewrites nothing indexes again, so the facts and resolved module names go before the files are linted.
    if plumbing.rewrote.is_none() {
        plumbing.ending.end(std::mem::take(index.facts));
        index.resolver.release(plumbing.ending);
    }
    let lint = |file: &RunFile, project: &Project, cache| {
        let first = file
            .first_parse
            .get()
            .and_then(|retained| retained.syntax.lock().expect("no worker panics while taking a parse").take());
        let mut linter = file_linter(file, project);
        linter.first_parse = first.as_ref().map(|(source, tree)| (source.as_str(), tree));
        work(&DiscoveredFile { path: file.path, linter, cache })
    };
    let mut passes = (0..files.len()).map(|_| Vec::new()).collect::<Vec<_>>();
    // Workers take the largest files first so the last large module does not delay the entire round.
    let mut order: Vec<_> = passes.iter_mut().enumerate().collect();
    order.sort_by_cached_key(|&(at, _)| {
        std::cmp::Reverse(
            file_caches[at]
                .map_or_else(|| files[at].path.metadata().map_or(0, |metadata| metadata.len()), |(_, key)| key.length),
        )
    });
    if files.iter().any(|file| file.first_parse.get().is_some()) {
        // Prefer the worker that allocated the tree. Idle workers can take a file from any queue.
        let count = rayon::current_num_threads();
        let mut queues = (0..=count).map(|_| Mutex::new(Vec::new())).collect::<Vec<_>>();
        for (at, result) in order.into_iter().rev() {
            let slot = files[at].first_parse.get().map_or(count, |parse| parse.worker);
            queues[slot].get_mut().expect("the queues are new").push((at, result));
        }
        rayon::broadcast(|thread| {
            let owner = thread.index();
            loop {
                let next = std::iter::once(owner)
                    .chain(std::iter::once(count))
                    .chain((1..count).map(|offset| (owner + offset) % count))
                    .find_map(|slot| queues[slot].lock().expect("no worker panics while taking a file").pop());
                let Some((at, result)) = next else { break };
                result.push(lint(&files[at], &project, file_caches[at]));
            }
        });
    } else {
        order
            .into_iter()
            .par_bridge()
            .for_each(|(at, passes)| passes.push(lint(&files[at], &project, file_caches[at])));
    }
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
        let collect = |at: usize| file_facts(&files[at], recache(at), parts, overlay, false);
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

fn file_linter<'a>(file: &'a RunFile<'a>, project: &'a Project) -> Linter<'a> {
    Linter::new(
        file.ruff,
        file.settings.clone(),
        &project.file(file.absolute.as_path()),
        file.package,
        file.module_path.clone(),
        file.sibling_init,
    )
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
    retain: bool,
) -> Option<FileFacts> {
    let (module_path, importable) = (file.module_path.as_deref(), file.importable.as_deref());
    let parts = parts.for_file(file.settings.is_test);
    let cached = cache.and_then(|(cache, key)| cache.facts(file.path, key));
    let parts = match cached {
        Some(facts) if facts.covers(parts, module_path, importable) => return Some(facts),
        Some(facts) => parts.with(facts.parts()),
        None => parts,
    };
    let source = overlay.source(file.path).ok()?;
    let facts = if retain && cache.is_some_and(|(_, key)| source.len() as u64 <= key.length) {
        let parsed = parse_source(&source, file.settings.from_ruff.target_version);
        let facts = if parsed.has_valid_syntax() {
            FileFacts::from_parse(file.path, &source, module_path, importable, parts, &parsed)
        } else {
            FileFacts::collect(file.path, &source, module_path, importable, parts)?
        };
        file.first_parse.get_or_init(|| {
            Box::new(WorkerParse {
                worker: rayon::current_thread_index().unwrap_or(0),
                syntax: Mutex::new(Some((source, parsed))),
            })
        });
        facts
    } else {
        FileFacts::collect(file.path, &source, module_path, importable, parts)?
    };
    if let Some((cache, key)) = cache {
        cache.set_facts(file.path, key, &facts);
    }
    Some(facts)
}

// Each file's `words` are read the first time a changed name needs them.
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
