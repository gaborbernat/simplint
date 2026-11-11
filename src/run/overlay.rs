use std::io;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use ruff_linter::source_kind::SourceKindDiff;
use rustc_hash::FxHashMap;

// The fixes of a `--diff` run, kept in memory since the run writes nothing: each file the fixes changed, with its
// source as read and its source after the fixes so far. A later round fixes the source after the fixes so far, which
// the disk does not hold.
#[derive(Default)]
pub struct Overlay(Mutex<FxHashMap<PathBuf, Fixed>>);

struct Fixed {
    original: String,
    current: String,
}

impl Overlay {
    fn current(&self, path: &Path) -> Option<String> {
        self.0.lock().expect("no worker panics while fixing").get(path).map(|fixed| fixed.current.clone())
    }

    pub fn source(&self, path: &Path) -> io::Result<String> {
        match self.current(path) {
            Some(current) => Ok(current),
            None => std::fs::read_to_string(path),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.0.lock().expect("no worker panics while fixing").is_empty()
    }

    // Records `fixed` as the source of `path` after a fix that changed `source`, and returns the diff from the source
    // as read to `fixed`.
    pub fn record(&self, path: &Path, source: &str, fixed: &str) -> String {
        let mut files = self.0.lock().expect("no worker panics while fixing");
        let entry = files
            .entry(path.to_path_buf())
            .or_insert_with(|| Fixed { original: source.to_owned(), current: String::new() });
        fixed.clone_into(&mut entry.current);
        SourceKindDiff::from_text(&entry.original, fixed, Some(path)).to_string()
    }
}
