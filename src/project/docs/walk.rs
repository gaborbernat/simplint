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

// The same walk as file discovery: ignore files apply when the settings respect them, and symlinks are not followed.
// It yields the pages, which Ruff's exclusions leave out, and every `conf.py`, which Sphinx reads whether or not Ruff
// lints it.
pub(super) fn pages(root: &Path, settings: &ruff_workspace::Settings, main: &Path) -> Vec<PathBuf> {
    let exclusions = Exclusions::of(settings);
    let main = main.to_path_buf();
    let walk = WalkBuilder::new(root)
        .standard_filters(settings.file_resolver.respect_gitignore)
        .hidden(false)
        .filter_entry(move |entry| {
            let path = entry.path();
            let name = path.file_name().unwrap_or_default();
            if entry.file_type().is_some_and(|kind| kind.is_dir()) {
                enters(path, name, &exclusions, &main)
            } else {
                !is_hidden(name)
            }
        })
        .build();
    let exclusions = Exclusions::of(settings);
    walk.filter_map(Result::ok)
        .filter(|entry| is_candidate(root, entry, &exclusions))
        .map(ignore::DirEntry::into_path)
        .collect()
}

struct Exclusions {
    exclude: FilePatternSet,
    extend_exclude: FilePatternSet,
}

impl Exclusions {
    fn of(settings: &ruff_workspace::Settings) -> Self {
        Self {
            exclude: settings.file_resolver.exclude.clone(),
            extend_exclude: settings.file_resolver.extend_exclude.clone(),
        }
    }

    fn excludes(&self, path: &Path, name: &OsStr) -> bool {
        match_exclusion(path, name, &self.exclude) || match_exclusion(path, name, &self.extend_exclude)
    }
}

// Only a directory can be a nested project root, so a file skips this check.
fn enters(path: &Path, name: &OsStr, exclusions: &Exclusions, main: &Path) -> bool {
    !is_hidden(name) && !exclusions.excludes(path, name) && !is_nested_root(main, path)
}

fn is_hidden(name: &OsStr) -> bool {
    name.to_str().is_some_and(|name| name.starts_with('.'))
}

// A page Ruff does not exclude, or a `conf.py`, which counts even where Ruff excludes it. A symlinked `conf.py` counts
// when its target lies inside the project, as Sphinx follows the link.
fn is_candidate(root: &Path, entry: &ignore::DirEntry, exclusions: &Exclusions) -> bool {
    let path = entry.path();
    let name = path.file_name().unwrap_or_default();
    entry.file_type().is_some_and(|kind| {
        if is_configuration(path) {
            kind.is_file() || (kind.is_symlink() && linked_inside(root, path))
        } else {
            kind.is_file()
                && is_page(path)
                && !exclusions.excludes(path, name)
                // Past 8 MiB a markup file is generated output, and reading it whole costs memory for nothing.
                && entry.metadata().is_ok_and(|metadata| metadata.len() <= 8 << 20)
        }
    })
}

fn linked_inside(root: &Path, path: &Path) -> bool {
    fs::canonicalize(path).ok().zip(fs::canonicalize(root).ok()).is_some_and(|(target, root)| target.starts_with(root))
}

fn is_page(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "rst" || extension == "md")
}

pub(super) fn is_configuration(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "conf.py")
}

// Directories a project's own `conf.py` does not belong to: vendored copies, and the ones a parent project's build
// keeps, which a project under the root is a separate project of.
const VENDORED: [&str; 4] = ["_vendor", "vendor", "third_party", "subprojects"];
const MARKERS: [&str; 3] = ["pyproject.toml", "setup.py", "setup.cfg"];

// Whether a `conf.py` under the root belongs to another project or to vendored code: one of the vendored directories
// is on its way, or a directory between it and the root holds a project marker.
pub(super) fn is_foreign(root: &Path, path: &Path) -> bool {
    path.ancestors().skip(1).take_while(|directory| *directory != root).any(|directory| {
        directory.file_name().is_some_and(|name| VENDORED.iter().any(|vendored| name == *vendored))
            || MARKERS.iter().any(|marker| directory.join(marker).is_file())
    })
}

// Files whose presence or content changes what the documentation reads, which the key stats along with the pages.
// The index files and project markers decide which `conf.py` counts and which project it belongs to.
const WATCHED: [&str; 6] = [".gitignore", ".ignore", "index.txt", "pyproject.toml", "setup.py", "setup.cfg"];

// Stats stand in for the ignore files' matchers, which cost more than listing directories. A directory's time moves
// when an entry is added or renamed but not when a page is edited, so pages and ignore files are stat'd too. Each
// level is listed on all threads, as a gitignored build tree can hold tens of thousands of directories. A directory an
// ignore file hides is listed too, so the key covers at least what the walk reads.
// `global` is git's excludes file, which git finds only through a home directory or its configuration.
pub(super) fn tree_key(root: &Path, settings: &ruff_workspace::Settings, main: &Path, global: Option<&Path>) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    settings.file_resolver.cache_key(&mut hasher);
    for ancestor in root.ancestors() {
        for name in [".gitignore", ".ignore", ".git/info/exclude"] {
            stat_key(&ancestor.join(name), &mut hasher);
        }
    }
    if let Some(global) = global {
        stat_key(global, &mut hasher);
    }
    let mut stats = Vec::new();
    let mut level = vec![root.to_path_buf()];
    let exclusions = Exclusions::of(settings);
    while !level.is_empty() {
        let listed: Vec<Listing> = level.par_iter().map(|directory| list(directory, &exclusions, main)).collect();
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

struct Listing {
    stats: Vec<(PathBuf, Option<Stat>)>,
    below: Vec<PathBuf>,
}

fn list(directory: &Path, exclusions: &Exclusions, main: &Path) -> Listing {
    let mut stats = vec![(directory.to_path_buf(), stat(directory))];
    let mut below = Vec::new();
    for entry in fs::read_dir(directory).into_iter().flatten().filter_map(Result::ok) {
        let path = entry.path();
        match listed(&path, &entry.file_name(), entry.file_type().ok(), exclusions, main) {
            Listed::Below => below.push(path),
            Listed::Stat => {
                let stat = stat(&path);
                stats.push((path, stat));
            }
            Listed::Skip => {}
        }
    }
    Listing { stats, below }
}

#[derive(Debug, PartialEq)]
enum Listed {
    Below,
    Stat,
    Skip,
}

// An entry whose type cannot be read was removed after the listing, which the directory's own stat records.
fn listed(path: &Path, name: &OsStr, file_type: Option<fs::FileType>, exclusions: &Exclusions, main: &Path) -> Listed {
    let Some(file_type) = file_type else {
        return Listed::Skip;
    };
    if file_type.is_dir() {
        return if enters(path, name, exclusions, main) { Listed::Below } else { Listed::Skip };
    }
    let stated = if is_configuration(path) {
        file_type.is_file() || file_type.is_symlink()
    } else {
        file_type.is_file() && (is_page(path) || WATCHED.iter().any(|watched| name == *watched))
    };
    if stated { Listed::Stat } else { Listed::Skip }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;

    use ruff_linter::settings::types::FilePatternSet;
    use tempfile::TempDir;

    use super::{Exclusions, Listed, list, listed, tree_key};

    #[test]
    fn global_excludes_file_is_part_of_the_key() {
        let folder = TempDir::new().expect("create a temporary directory");
        let global = folder.path().join("excludes");
        fs::write(&global, "*.log\n").expect("write the excludes file");
        let settings = ruff_workspace::Settings::default();
        let root = folder.path();
        assert_ne!(tree_key(root, &settings, root, None), tree_key(root, &settings, root, Some(&global)));
    }

    #[test]
    fn entry_whose_type_cannot_be_read_is_skipped() {
        let exclusions = Exclusions { exclude: FilePatternSet::default(), extend_exclude: FilePatternSet::default() };
        assert_eq!(listed(Path::new("conf.py"), OsStr::new("conf.py"), None, &exclusions, Path::new("")), Listed::Skip);
    }

    // Sphinx reads a `conf.py` a link points at, but not one that is a socket or another special file.
    #[cfg(unix)]
    #[test]
    fn configuration_counts_as_a_file_or_a_link_only() {
        let folder = TempDir::new().expect("create a temporary directory");
        let (linked, socket) = (folder.path().join("linked"), folder.path().join("socket"));
        fs::create_dir(&linked).expect("create the linked directory");
        fs::create_dir(&socket).expect("create the socket directory");
        fs::write(folder.path().join("real.py"), "").expect("write the link target");
        std::os::unix::fs::symlink(folder.path().join("real.py"), linked.join("conf.py")).expect("link conf.py");
        let _listener = std::os::unix::net::UnixListener::bind(socket.join("conf.py")).expect("bind a socket conf.py");
        let exclusions = Exclusions { exclude: FilePatternSet::default(), extend_exclude: FilePatternSet::default() };
        let stated = |directory: &Path| -> Vec<_> {
            list(directory, &exclusions, folder.path()).stats.into_iter().map(|(path, _)| path).collect()
        };
        assert_eq!(
            (stated(&linked), stated(&socket)),
            (vec![linked.clone(), linked.join("conf.py")], vec![socket.clone()])
        );
    }
}
