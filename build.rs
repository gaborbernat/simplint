// Names the cache directory after the source the cached data depends on, so changing it starts a fresh cache rather
// than failing to read the old one, or reading facts and findings an older build produced, as happens between
// development builds of one version. The findings depend on every rule and on the code that runs them, so the name
// hashes every Rust file of the crate: any change starts a fresh cache, and a new module cannot be left out.
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
        file.hash(&mut hasher);
        fs::read_to_string(&file).unwrap_or_else(|error| panic!("read {}: {error}", file.display())).hash(&mut hasher);
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
