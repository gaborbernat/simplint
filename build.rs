// Names the cache directory after the source the cached data depends on, so changing it starts a fresh cache rather
// than failing to read the old one, or reading facts an older build collected, as happens between development builds
// of one version. That source is every file declaring a type the cache stores, every file of `src/project`, which
// collects the facts, and the files outside it whose helpers decide what the facts hold. Tracking imports would follow
// every change, but a fixed list is simpler; a new helper module the facts read belongs in `SHARED`.
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

const SHARED: [&str; 2] = ["src/syntax.rs", "src/lint/noqa.rs"];

fn main() {
    println!("cargo::rerun-if-changed=src");
    let mut files = Vec::new();
    collect(Path::new("src"), &mut files);
    files.sort();
    let mut hasher = DefaultHasher::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
        let feeds_facts = file.starts_with("src/project") || SHARED.iter().any(|shared| file == Path::new(shared));
        if feeds_facts || text.contains("rkyv::Archive") {
            text.hash(&mut hasher);
        }
    }
    println!("cargo::rustc-env=SIMPLINT_CACHE_FORMAT={:016x}", hasher.finish());
}

fn collect(directory: &Path, files: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(directory).unwrap_or_else(|error| panic!("read {}: {error}", directory.display()));
    for entry in entries {
        let path = entry.unwrap_or_else(|error| panic!("read {}: {error}", directory.display())).path();
        if path.is_dir() {
            collect(&path, files);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
}
