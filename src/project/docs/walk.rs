use std::ffi::OsStr;
use std::fs;
use std::hash::Hasher;
use std::path::{Path, PathBuf};

use ignore::WalkBuilder;
use rayon::prelude::*;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::settings::types::FilePatternSet;
use ruff_workspace::resolver::match_exclusion;

use crate::project::cache::{Stat, stat, stat_key};
use crate::settings::is_nested_root;

// The `.rst` and `.md` pages under a root the walk reaches.
pub(super) fn pages(root: &Path, settings: &ruff_workspace::Settings, main: &Path) -> Vec<PathBuf> {
    let exclude = settings.file_resolver.exclude.clone();
    let extend_exclude = settings.file_resolver.extend_exclude.clone();
    let main = main.to_path_buf();
    let walk = WalkBuilder::new(root)
        .standard_filters(settings.file_resolver.respect_gitignore)
        .hidden(false)
        .filter_entry(move |entry| {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default();
            if entry.file_type().is_some_and(|kind| kind.is_dir()) {
                enters(path, name, &exclude, &extend_exclude, &main)
            } else {
                is_kept(path, name, &exclude, &extend_exclude)
            }
        })
        .build();
    walk.filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|file_type| file_type.is_file()) && is_page(entry.path()))
        // A markup file past 8 MiB is generated output rather than prose someone wrote, and reading it whole costs
        // memory for nothing.
        .filter(|entry| entry.metadata().is_ok_and(|metadata| metadata.len() <= 8 << 20))
        .map(ignore::DirEntry::into_path)
        .collect()
}

fn is_page(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rst" || extension == "md")
}

// Whether the walk goes into a directory: only a directory can be a nested project root, so a file skips that check
// and its stats.
fn enters(path: &Path, name: &OsStr, exclude: &FilePatternSet, extend_exclude: &FilePatternSet, main: &Path) -> bool {
    is_kept(path, name, exclude, extend_exclude) && !is_nested_root(main, path)
}

fn is_kept(path: &Path, name: &OsStr, exclude: &FilePatternSet, extend_exclude: &FilePatternSet) -> bool {
    !name.to_str().is_some_and(|name| name.starts_with('.'))
        && !match_exclusion(path, name, exclude)
        && !match_exclusion(path, name, extend_exclude)
}

// A key of what the walk's outcome under a root depends on, read without building the matchers of its ignore files,
// which cost more than listing the directories: the modification time of each directory the walk may enter, which an
// added, removed or renamed entry moves, the modification time and size of each page and each ignore file, whose edits
// leave their directory's time as it was, those of the ignore files above the root and the global one, which the walk
// reads too, and the walk's settings. A directory an ignore file hides is listed all the same, so the key covers at
// least what the walk reads. Each level of the tree is listed on all threads, as a gitignored build tree can hold tens
// of thousands of directories.
pub(super) fn tree_key(root: &Path, settings: &ruff_workspace::Settings, main: &Path) -> u64 {
    let resolver = &settings.file_resolver;
    let mut hasher = CacheKeyHasher::new();
    resolver.cache_key(&mut hasher);
    for ancestor in root.ancestors() {
        for name in [".gitignore", ".ignore", ".git/info/exclude"] {
            stat_key(&ancestor.join(name), &mut hasher);
        }
    }
    if let Some(global) = ignore::gitignore::gitconfig_excludes_path() {
        stat_key(&global, &mut hasher);
    }
    let mut stats = Vec::new();
    let mut level = vec![root.to_path_buf()];
    while !level.is_empty() {
        let listed: Vec<Listing> = level.par_iter().map(|directory| list(directory, settings, main)).collect();
        level = Vec::new();
        for Listing { stats: found, below } in listed {
            stats.extend(found);
            level.extend(below);
        }
    }
    stats.sort_unstable_by(|(left, _), (right, _)| left.cmp(right));
    for (path, stat) in stats {
        path.cache_key(&mut hasher);
        stat.cache_key(&mut hasher);
    }
    hasher.finish()
}

// What `tree_key` reads from one directory: its own stat and those of the pages and ignore files in it, and the
// directories below it the walk may enter.
struct Listing {
    stats: Vec<(PathBuf, Option<Stat>)>,
    below: Vec<PathBuf>,
}

fn list(directory: &Path, settings: &ruff_workspace::Settings, main: &Path) -> Listing {
    let resolver = &settings.file_resolver;
    let mut stats = vec![(directory.to_path_buf(), stat(directory))];
    let mut below = Vec::new();
    for entry in fs::read_dir(directory).into_iter().flatten().filter_map(Result::ok) {
        let (path, name) = (entry.path(), entry.file_name());
        let Ok(file_type) = entry.file_type() else { continue };
        if file_type.is_dir() {
            if enters(&path, &name, &resolver.exclude, &resolver.extend_exclude, main) {
                below.push(path);
            }
        } else if file_type.is_file() && (is_page(&path) || name == ".gitignore" || name == ".ignore") {
            let stat = stat(&path);
            stats.push((path, stat));
        }
    }
    Listing { stats, below }
}
