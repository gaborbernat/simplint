mod source;

use std::fs;
use std::hash::Hasher;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use anyhow::Result;
use filetime::FileTime;
use log::error;
use rkyv::util::AlignedVec;
use ruff_cache::{CacheKey, CacheKeyHasher};
use tempfile::NamedTempFile;

// A generator or copy can preserve both timestamp and size while changing the source.
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
    pub fn of(path: &Path, checked: FileTime) -> Option<Self> {
        let mut options = fs::File::options();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            use windows_sys::Win32::Storage::FileSystem::{FILE_SHARE_DELETE, FILE_SHARE_READ};
            // A writer can keep its last journal record open until it closes the handle.
            options.share_mode(FILE_SHARE_READ | FILE_SHARE_DELETE);
        }
        let (mut file, metadata) = options.open(path).and_then(|file| file.metadata().map(|meta| (file, meta))).ok()?;
        let mut hasher = CacheKeyHasher::new();
        FileTime::from_last_modification_time(&metadata).cache_key(&mut hasher);
        if !source::change_key(&file, &metadata, checked, &mut hasher) {
            let mut bytes = Vec::new();
            file.read_to_end(&mut bytes).ok()?;
            hasher.write(&bytes);
        }
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

// The cached archive is read in place from aligned memory.
pub fn read_aligned(path: &Path) -> io::Result<AlignedVec<16>> {
    let file = fs::File::open(path)?;
    let size = file.metadata().map(|metadata| metadata.len());
    read_archive(file, size)
}

fn read_archive(mut reader: impl Read, size: io::Result<u64>) -> io::Result<AlignedVec<16>> {
    // Leave room for the EOF read so a full buffer does not double in size just to finish reading.
    let size = size.unwrap_or_default().min(64 * 1024 * 1024);
    let mut bytes = AlignedVec::<16>::with_capacity(usize::try_from(size).expect("64 MiB fits usize") + 1);
    bytes.extend_from_reader(&mut reader)?;
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

// Create the cache before hashing the documentation tree. Creating it later can change the project directory's
// timestamp and invalidate the new key.
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
    // Development builds can change the cache format without changing the package version.
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

// Other versions may still be in use, for example by pre-commit. Age each build by its newest file because an in-place
// write can leave the directory timestamp unchanged. An empty directory may belong to a build that just started.
fn remove_stale(current: &Path) {
    let Some(Ok(siblings)) = current.parent().map(fs::read_dir) else {
        return;
    };
    for entry in siblings.filter_map(Result::ok) {
        let sibling = entry.path();
        if sibling == current || !is_build_directory(&entry) {
            continue;
        }
        // Another process may hold the directory open. A later run can retry removal.
        if last_written(&sibling)
            .is_none_or(|modified| SystemTime::now().duration_since(modified).is_ok_and(|age| age > MAX_LAST_SEEN))
        {
            let _ = fs::remove_dir_all(&sibling);
        }
    }
}

// Fall back to the directory timestamp when no file timestamp is available.
fn last_written(directory: &Path) -> Option<SystemTime> {
    newest(fs::read_dir(directory).into_iter().flatten()).or_else(|| directory.metadata().ok()?.modified().ok())
}

fn newest<E: Modified>(entries: impl Iterator<Item = io::Result<E>>) -> Option<SystemTime> {
    entries.filter_map(|entry| entry.ok()?.modified().ok()).max()
}

trait Modified {
    fn modified(&self) -> io::Result<SystemTime>;
}

impl Modified for fs::DirEntry {
    fn modified(&self) -> io::Result<SystemTime> {
        self.metadata()?.modified()
    }
}

// Restrict cleanup to recognized build directories containing only cache files.
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

// Readers must see a complete archive even if another process is writing. The first write can precede package cache
// initialization.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path.parent().expect("a cache file sits in a directory");
    fs::create_dir_all(directory)?;
    let mut temporary = tempfile_in(directory)?;
    temporary.write_all(bytes)?;
    temporary.persist(path)?;
    Ok(())
}

// Honor the process umask for shared caches. NamedTempFile defaults to mode 0600.
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

    use super::{Modified, is_build_name, last_written, newest, read_aligned, read_archive, write_atomically};

    #[test_case("0.11.1-0123456789abcdef", true ; "release")]
    #[test_case("0.11.1+local.dev-0123456789abcdef", true ; "local_version")]
    #[test_case("0.11_1-0123456789abcdef", false ; "underscore_in_version")]
    #[test_case("-0123456789abcdef", false ; "no_version")]
    #[test_case("0.11.1-0123456789ABCDEF", false ; "uppercase_hash")]
    fn build_directory_names(name: &str, build: bool) {
        assert_eq!(is_build_name(name), build);
    }

    #[test_case(Err(std::io::ErrorKind::Unsupported) ; "unavailable")]
    #[test_case(Ok(2) ; "underestimate")]
    fn cache_reads_all_bytes_without_an_exact_size(size: Result<u64, std::io::ErrorKind>) {
        let bytes = read_archive(b"archive".as_slice(), size.map_err(std::io::Error::from)).expect("read archive");
        assert_eq!(bytes.as_slice(), b"archive");
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
        let initialized = super::init(&ruff_cache, &root.path().join("build"));
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

    #[cfg(unix)]
    #[test]
    fn writing_a_file_that_cannot_grow_fails() {
        crate::limits::limited(
            concat!(module_path!(), "::writing_a_file_that_cannot_grow_fails"),
            crate::limits::Limit::FileSize(0),
            || TempDir::new().expect("create a temporary directory"),
            |root| {
                let target = root.path().join("cache");
                let error = write_atomically(&target, b"data").expect_err("the file cannot grow");
                assert_eq!(
                    (error.downcast_ref::<std::io::Error>().map(std::io::Error::kind), target.exists()),
                    (Some(std::io::ErrorKind::FileTooLarge), false)
                );
            },
        );
    }

    #[cfg(unix)]
    #[test]
    fn directory_without_search_permission_goes_by_its_own_time() {
        use std::os::unix::fs::PermissionsExt;

        use filetime::{FileTime, set_file_mtime};

        let root = TempDir::new().expect("create a temporary directory");
        let directory = root.path().join("0.0.1-00000000000000a1");
        fs::create_dir(&directory).expect("create the directory");
        fs::write(directory.join("1"), "").expect("write a cache file");
        set_file_mtime(directory.join("1"), FileTime::from_unix_time(1_000, 0)).expect("age the file");
        set_file_mtime(&directory, FileTime::from_unix_time(2_000, 0)).expect("age the directory");
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o444)).expect("make the directory unsearchable");
        let written = last_written(&directory);
        fs::set_permissions(&directory, fs::Permissions::from_mode(0o755)).expect("make the directory searchable");

        assert_eq!(written.map(FileTime::from_system_time), Some(FileTime::from_unix_time(2_000, 0)));
    }

    #[test]
    fn directory_removed_since_listing_has_no_time() {
        let root = TempDir::new().expect("create a temporary directory");
        assert_eq!(last_written(&root.path().join("0.0.1-00000000000000a1")), None);
    }

    struct Written(std::time::SystemTime);

    impl Modified for Written {
        fn modified(&self) -> std::io::Result<std::time::SystemTime> {
            Ok(self.0)
        }
    }

    #[test]
    fn entry_the_listing_failed_on_is_skipped() {
        let time = std::time::SystemTime::UNIX_EPOCH;
        let entries = [Err(std::io::Error::other("disk error")), Ok(Written(time))];
        assert_eq!(newest(entries.into_iter()), Some(time));
    }
}
