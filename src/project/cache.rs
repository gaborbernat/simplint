use std::fs;
use std::hash::Hasher;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Result;
use filetime::FileTime;
use log::error;
use rkyv::util::AlignedVec;
use ruff_cache::{CacheKey, CacheKeyHasher};
use tempfile::NamedTempFile;

// A file's modification time, permissions and size. Ruff's `FileCacheKey` hashes the first two; the size also catches a
// rewrite within one tick of a coarse clock, or a copy that keeps the modification time. The size also bounds the
// ranges of the facts cached under the key.
#[derive(Clone, Copy)]
pub struct FileKey {
    pub hash: u64,
    pub length: u64,
}

impl CacheKey for FileKey {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.hash.cache_key(state);
    }
}

impl FileKey {
    pub fn of(path: &Path) -> Option<Self> {
        let metadata = path.metadata().ok()?;
        let mut hasher = CacheKeyHasher::new();
        FileTime::from_last_modification_time(&metadata).cache_key(&mut hasher);
        permissions(&metadata).cache_key(&mut hasher);
        metadata.len().cache_key(&mut hasher);
        Some(Self { hash: hasher.finish(), length: metadata.len() })
    }
}

#[cfg(unix)]
fn permissions(metadata: &fs::Metadata) -> u32 {
    use std::os::unix::fs::PermissionsExt;
    metadata.permissions().mode()
}

#[cfg(not(unix))]
fn permissions(metadata: &fs::Metadata) -> u32 {
    u32::from(metadata.permissions().readonly())
}

// rkyv reads an archive in place, which needs its bytes aligned to 16.
pub fn read_aligned(path: &Path) -> io::Result<AlignedVec<16>> {
    let mut file = fs::File::open(path)?;
    let mut bytes = AlignedVec::<16>::new();
    bytes.extend_from_reader(&mut file)?;
    Ok(bytes)
}

pub(super) type Stat = (i64, u32, u64);

pub(super) fn stat(path: &Path) -> Option<Stat> {
    fs::metadata(path).ok().map(|metadata| {
        let modified = FileTime::from_last_modification_time(&metadata);
        (modified.unix_seconds(), modified.nanoseconds(), metadata.len())
    })
}

pub(super) fn stat_key(path: &Path, hasher: &mut CacheKeyHasher) {
    path.cache_key(hasher);
    stat(path).cache_key(hasher);
}

// Makes this build's cache directory and drops other builds' directories. The documentation key hashes the times of
// the directories it walks, and the cache directory sits in a project root by default, so creating it while that walk
// runs would change a key that is then stored, and the next run would miss it.
pub fn prepare(ruff_cache_dir: &Path) {
    let directory = build_directory(ruff_cache_dir);
    if let Err(error) = init(ruff_cache_dir, &directory) {
        error!("Failed to initialize cache at {}: {error:?}", ruff_cache_dir.display());
    }
    remove_stale(&directory);
}

// Ruff's retention for unused cache entries, applied to whole build directories too.
pub const MAX_LAST_SEEN: Duration = Duration::from_hours(720);

pub fn build_directory(ruff_cache_dir: &Path) -> PathBuf {
    // The version and a hash of the crate's source, which `build.rs` computes, so a development build that changes it
    // does not read caches an earlier build of the same version wrote.
    const FORMAT: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("SIMPLINT_CACHE_FORMAT"));
    directory(ruff_cache_dir).join(FORMAT)
}

pub fn directory(ruff_cache_dir: &Path) -> PathBuf {
    ruff_cache_dir.join("simplint")
}

fn init(ruff_cache_dir: &Path, directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    cachedir::ensure_tag(ruff_cache_dir)?;
    match fs::OpenOptions::new().write(true).create_new(true).open(ruff_cache_dir.join(".gitignore")) {
        Ok(mut file) => file.write_all(b"# Automatically created by ruff.\n*\n"),
        Err(error) => (error.kind() == io::ErrorKind::AlreadyExists).then_some(()).ok_or(error),
    }
}

// Removes the directories other simplint builds wrote that no run has written to for `MAX_LAST_SEEN`. A directory
// another version still writes to stays, as when pre-commit pins one version and the command line runs another. Its
// newest file tells its age, since Windows does not move a directory's modification time when a file in it changes; an
// empty one, as another run creates before it writes its cache, goes by its own.
fn remove_stale(current: &Path) {
    let Some(Ok(siblings)) = current.parent().map(fs::read_dir) else {
        return;
    };
    for entry in siblings.filter_map(Result::ok) {
        let sibling = entry.path();
        if sibling == current || !is_build_directory(&entry) {
            continue;
        }
        let newest = fs::read_dir(&sibling)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.and_then(|entry| entry.metadata()).and_then(|metadata| metadata.modified()).ok())
            .max()
            .or_else(|| sibling.metadata().and_then(|metadata| metadata.modified()).ok());
        // Removal is best effort: a directory it cannot remove, as one another process has open, waits for a later run.
        if newest.is_none_or(|modified| SystemTime::now().duration_since(modified).is_ok_and(|age| age > MAX_LAST_SEEN))
        {
            let _ = fs::remove_dir_all(&sibling);
        }
    }
}

// Only a real directory named as a build, holding nothing but cache files, is removed, so a directory a user put in the
// simplint directory stays.
fn is_build_directory(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(is_build_name)
        && entry.file_type().is_ok_and(|file_type| file_type.is_dir())
        && holds_only_cache_files(&entry.path())
}

fn is_build_name(name: &str) -> bool {
    name.rsplit_once('-').is_some_and(|(version, hash)| {
        !version.is_empty()
            && version.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
            && hash.len() == 16
            && hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

fn holds_only_cache_files(directory: &Path) -> bool {
    fs::read_dir(directory).is_ok_and(|mut entries| {
        entries.all(|entry| {
            entry.is_ok_and(|entry| {
                entry.file_type().is_ok_and(|file_type| file_type.is_file())
                    && entry
                        .file_name()
                        .to_str()
                        .is_some_and(|name| !name.is_empty() && name.bytes().all(|byte| byte.is_ascii_digit()))
            })
        })
    })
}

pub fn documentation_file(ruff_cache_dir: &Path, project_root: &Path) -> PathBuf {
    let mut hasher = CacheKeyHasher::new();
    "documentation".cache_key(&mut hasher);
    project_root.cache_key(&mut hasher);
    build_directory(ruff_cache_dir).join(hasher.finish().to_string())
}

// Writes a cache file through a temporary file next to it, so a reader never sees it cut short. The directory is made
// first, as the run may write before it has opened the package caches.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path.parent().expect("a cache file sits in a directory");
    fs::create_dir_all(directory)?;
    tempfile_in(directory).and_then(|mut temporary| temporary.write_all(bytes).map(|()| temporary))?.persist(path)?;
    Ok(())
}

// A temporary file with the permissions of a file created the usual way, as `NamedTempFile` defaults to `0o600`.
#[cfg(unix)]
pub fn tempfile_in(path: &Path) -> io::Result<NamedTempFile> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new().permissions(fs::Permissions::from_mode(0o666)).tempfile_in(path)
}

#[cfg(not(unix))]
pub fn tempfile_in(path: &Path) -> io::Result<NamedTempFile> {
    tempfile::Builder::new().tempfile_in(path)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;
    use test_case::test_case;

    use super::{init, is_build_name, read_aligned, write_atomically};

    #[test_case("0.11.1-0123456789abcdef", true ; "release")]
    #[test_case("0.11.1+local.dev-0123456789abcdef", true ; "local_version")]
    #[test_case("0.11_1-0123456789abcdef", false ; "underscore_in_version")]
    #[test_case("-0123456789abcdef", false ; "no_version")]
    #[test_case("0.11.1-0123456789ABCDEF", false ; "uppercase_hash")]
    fn build_directory_names(name: &str, build: bool) {
        assert_eq!(is_build_name(name), build);
    }

    #[test]
    fn reading_a_directory_as_a_cache_fails() {
        let directory = TempDir::new().expect("create a temporary directory");

        assert!(read_aligned(directory.path()).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_read_only_ruff_cache_fails_the_initialization() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempDir::new().expect("create a temporary directory");
        let ruff_cache = root.path().join("ruff");
        fs::create_dir(&ruff_cache).expect("create the directory");
        fs::set_permissions(&ruff_cache, fs::Permissions::from_mode(0o555)).expect("make the directory read-only");
        let initialized = init(&ruff_cache, &root.path().join("build"));
        fs::set_permissions(&ruff_cache, fs::Permissions::from_mode(0o755)).expect("make the directory writable");

        assert!(initialized.is_err());
    }

    #[test]
    fn writing_over_a_directory_fails() {
        let root = TempDir::new().expect("create a temporary directory");
        let target = root.path().join("cache");
        fs::create_dir(&target).expect("create the directory");

        assert!(write_atomically(&target, b"data").is_err());
    }

    #[cfg(unix)]
    #[test]
    fn writing_into_a_read_only_directory_fails() {
        use std::os::unix::fs::PermissionsExt;

        let root = TempDir::new().expect("create a temporary directory");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o555)).expect("make the directory read-only");
        let written = write_atomically(&root.path().join("cache"), b"data");
        fs::set_permissions(root.path(), fs::Permissions::from_mode(0o755)).expect("make the directory writable");

        assert!(written.is_err());
    }
}
