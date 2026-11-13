use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use ruff_cache::{CacheKey, CacheKeyHasher};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::project::FileFacts;
use crate::syntax::name_prefix;

use super::{FileId, FileIds};

#[derive(Clone, Copy, PartialEq)]
pub struct PrefixSiblings {
    pub others: usize,
    // The extension of a `_<prefix>` module beside the group, which a new package would shadow.
    pub prefix_module: Option<&'static str>,
}

impl CacheKey for PrefixSiblings {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.others.cache_key(state);
        self.prefix_module.cache_key(state);
    }
}

// A stub counts toward its prefix without being reported. A `_<prefix>/` directory is a package when the run holds a
// file in it or an `__init__.py` sits on disk, so a package the run leaves out still counts.
pub(super) fn prefix_siblings(indexed: &[(FileId, &FileFacts)], files: &FileIds) -> FxHashMap<FileId, PrefixSiblings> {
    let mut directories: FxHashSet<&Path> = FxHashSet::default();
    let mut groups: FxHashMap<(&Path, &str), FxHashSet<&str>> = FxHashMap::default();
    for (id, _) in indexed {
        let path = files.path(*id);
        directories.extend(path.ancestors());
        let (Some(directory), Some(stem)) = (path.parent(), path.file_stem().and_then(OsStr::to_str)) else {
            continue;
        };
        if let Some(prefix) = name_prefix(stem) {
            groups.entry((directory, prefix)).or_default().insert(stem);
        }
    }
    indexed
        .iter()
        .filter_map(|(id, _)| {
            let path = files.path(*id);
            if path.extension() != Some(OsStr::new("py")) {
                return None;
            }
            let stem = path.file_stem().expect("a file with an extension has a stem");
            let prefix = name_prefix(stem.to_str()?)?;
            let directory = path.parent().expect("a run file sits in a directory");
            // The loop above grouped every file with a prefix, this one among them.
            let modules = groups[&(directory, prefix)].len();
            let package = directory.join(format!("_{prefix}"));
            if modules < 3 || directories.contains(package.as_path()) || package.join("__init__.py").is_file() {
                return None;
            }
            let prefix_module =
                ["py", "pyi"].into_iter().find(|extension| directory.join(format!("_{prefix}.{extension}")).is_file());
            Some((*id, PrefixSiblings { others: modules - 1, prefix_module }))
        })
        .collect()
}

// The file names of the run in each directory that holds a test module, which tell whether pytest collects from it.
pub(super) fn test_directory_modules(
    indexed: &[(FileId, &FileFacts)],
    test_modules: &FxHashSet<FileId>,
    files: &FileIds,
) -> FxHashMap<PathBuf, Vec<String>> {
    let directories: FxHashSet<&Path> = test_modules.iter().filter_map(|id| files.path(*id).parent()).collect();
    let mut modules: FxHashMap<PathBuf, Vec<String>> = FxHashMap::default();
    for (id, _) in indexed {
        let path = files.path(*id);
        if let (Some(directory), Some(name)) = (path.parent(), path.file_name().and_then(OsStr::to_str))
            && directories.contains(directory)
        {
            modules.entry(directory.to_path_buf()).or_default().push(name.to_owned());
        }
    }
    for names in modules.values_mut() {
        names.sort_unstable();
    }
    modules
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use crate::project::{FileFacts, Parts};

    use super::{FileIds, prefix_siblings};

    // A name that is not UTF-8 has no prefix to group by, so only the readable siblings count toward theirs.
    #[cfg(unix)]
    #[test]
    fn file_name_that_is_not_utf8_joins_no_group() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        let facts = FileFacts::collect(Path::new("/project/a.py"), "", None, None, Parts::default()).expect("facts");
        let mut ids = Arc::new(FileIds::default());
        let indexed: Vec<_> = [b"/project/_cli_one.py".as_slice(), b"/project/_cli_two.py", b"/project/_cli_\xff.py"]
            .into_iter()
            .map(|path| (FileIds::intern(&mut ids, Path::new(OsStr::from_bytes(path))), &facts))
            .collect();
        assert!(prefix_siblings(&indexed, &ids).is_empty());
    }
}
