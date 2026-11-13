use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard};

use ruff_linter::source_kind::SourceKindDiff;
use rustc_hash::FxHashMap;

// Kept in memory because a `--diff` run writes nothing, so a later round reads the source after the earlier fixes,
// which the disk does not hold.
#[derive(Default)]
pub(crate) struct Overlay(Mutex<FxHashMap<PathBuf, Fixed>>);

struct Fixed {
    original: String,
    current: String,
}

impl Overlay {
    fn current(&self, path: &Path) -> Option<String> {
        self.files().get(path).map(|fixed| fixed.current.clone())
    }

    pub(crate) fn source(&self, path: &Path) -> io::Result<String> {
        match self.current(path) {
            Some(current) => Ok(current),
            None => std::fs::read_to_string(path),
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.files().is_empty()
    }

    pub(crate) fn record(&self, path: &Path, source: &str, fixed: &str) -> String {
        let mut files = self.files();
        let entry = files
            .entry(path.to_path_buf())
            .or_insert_with(|| Fixed { original: source.to_owned(), current: String::new() });
        fixed.clone_into(&mut entry.current);
        SourceKindDiff::from_text(&entry.original, fixed, Some(path)).to_string()
    }

    fn files(&self) -> MutexGuard<'_, FxHashMap<PathBuf, Fixed>> {
        self.0.lock().expect("no worker panics while fixing")
    }
}
