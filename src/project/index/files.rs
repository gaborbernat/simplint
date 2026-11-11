use rustc_hash::FxHashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

// A run file by a number, so the index keys its maps by copies of it instead of hashing its path.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct FileId(u32);

// Every path the index has seen, numbered in the order they were first seen. Ids stay valid for the whole run, as a
// fix round indexes the same files again.
#[derive(Clone, Default)]
pub struct FileIds {
    pub(super) ids: FxHashMap<PathBuf, FileId>,
    pub(super) paths: Vec<PathBuf>,
}

impl FileIds {
    // Paths are absolute, as the run hands them over, so a lookup hashes the path as it is.
    // A path that was never interned has no entries in the index.
    pub(super) fn get(&self, path: &Path) -> Option<FileId> {
        self.ids.get(path).copied()
    }

    pub(super) fn path(&self, id: FileId) -> &Path {
        &self.paths[id.0 as usize]
    }

    // Copy on write, so a project that still holds the table keeps its ids; numbering only grows when a path is new.
    pub(super) fn intern(files: &mut Arc<Self>, path: &Path) -> FileId {
        if let Some(id) = files.ids.get(path) {
            return *id;
        }
        let files = Arc::make_mut(files);
        let id = FileId(u32::try_from(files.paths.len()).expect("the run has fewer files than a FileId counts"));
        files.ids.insert(path.to_path_buf(), id);
        files.paths.push(path.to_path_buf());
        id
    }
}
