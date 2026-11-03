// Names the cache directory after the source of every type the cache stores, so changing one starts a fresh cache rather
// than failing to read the old one, as happens between development builds of one version.
use std::fs;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::path::{Path, PathBuf};

fn main() {
    println!("cargo::rerun-if-changed=src");
    let mut files = Vec::new();
    collect(Path::new("src"), &mut files);
    files.sort();
    let mut hasher = DefaultHasher::new();
    for file in files {
        let text = fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display()));
        if text.contains("rkyv::Archive") {
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
