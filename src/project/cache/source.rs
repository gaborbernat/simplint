use std::fs::{File, Metadata};

use filetime::FileTime;

#[cfg(any(target_os = "linux", target_os = "macos", windows))]
use ruff_cache::CacheKey;
use ruff_cache::CacheKeyHasher;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub(super) fn change_key(file: &File, metadata: &Metadata, checked: FileTime, hasher: &mut CacheKeyHasher) -> bool {
    use std::os::unix::fs::MetadataExt;

    // Two writes can share a clock tick. Hash the bytes until that tick has passed.
    if checked.unix_seconds().saturating_sub(metadata.ctime()) < 2 || !local_change_times(file) {
        return false;
    }
    (metadata.dev(), metadata.ino(), metadata.ctime(), metadata.ctime_nsec()).cache_key(hasher);
    true
}

#[cfg(target_os = "linux")]
fn local_change_times(file: &File) -> bool {
    rustix::fs::fstatfs(file).is_ok_and(|fs| {
        matches!(
            fs.f_type,
            libc::BTRFS_SUPER_MAGIC | libc::EXT4_SUPER_MAGIC | libc::TMPFS_MAGIC | libc::XFS_SUPER_MAGIC
        )
    })
}

#[cfg(target_os = "macos")]
fn local_change_times(file: &File) -> bool {
    rustix::fs::fstatfs(file).is_ok_and(|fs| fs.f_fstypename.starts_with(&b"apfs\0".map(u8::cast_signed)))
}

#[cfg(windows)]
pub(super) fn change_key(file: &File, metadata: &Metadata, _: FileTime, hasher: &mut CacheKeyHasher) -> bool {
    use std::mem::size_of_val;
    use std::os::windows::fs::MetadataExt;
    use std::os::windows::io::AsRawHandle;

    use windows_sys::Win32::Storage::FileSystem::{BY_HANDLE_FILE_INFORMATION, GetFileInformationByHandle};
    use windows_sys::Win32::System::IO::DeviceIoControl;
    use windows_sys::Win32::System::Ioctl::{FSCTL_READ_FILE_USN_DATA, READ_FILE_USN_DATA, USN_RECORD_V2};

    let version = READ_FILE_USN_DATA { MinMajorVersion: 2, MaxMajorVersion: 2 };
    // The record includes a variable-length UTF-16 file name. An aligned buffer fits an NTFS component.
    let mut buffer = [0u64; 128];
    let mut returned = 0;
    let mut info = BY_HANDLE_FILE_INFORMATION::default();
    // The handle stays open and each buffer has the size and alignment required by its type.
    let ok = unsafe {
        DeviceIoControl(
            file.as_raw_handle(),
            FSCTL_READ_FILE_USN_DATA,
            (&raw const version).cast(),
            size_of::<READ_FILE_USN_DATA>() as u32,
            buffer.as_mut_ptr().cast(),
            size_of_val(&buffer) as u32,
            &raw mut returned,
            std::ptr::null_mut(),
        ) != 0
            && GetFileInformationByHandle(file.as_raw_handle(), &raw mut info) != 0
    };
    if !ok || (returned as usize) < std::mem::offset_of!(USN_RECORD_V2, FileName) {
        return false;
    }
    // Only the fixed fields of the requested V2 record are read.
    let record = unsafe { &*buffer.as_ptr().cast::<USN_RECORD_V2>() };
    if record.MajorVersion != 2 || record.Usn == 0 {
        return false;
    }
    (info.dwVolumeSerialNumber, metadata.creation_time(), record.FileReferenceNumber, record.Usn).cache_key(hasher);
    true
}

#[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
pub(super) fn change_key(_: &File, _: &Metadata, _: FileTime, _: &mut CacheKeyHasher) -> bool {
    false
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use std::fs::{self, File};
    use std::os::unix::fs::MetadataExt;

    use filetime::FileTime;

    use super::local_change_times;
    use crate::project::cache::FileKey;

    #[test]
    fn an_unreadable_source_has_no_content_key() {
        let dir = tempfile::tempdir().expect("temporary directory");
        assert!(FileKey::of(dir.path(), FileTime::now()).is_none());
    }

    #[test]
    fn identical_replacement_uses_a_new_inode_key_after_the_clock_tick() {
        let dir = tempfile::tempdir().expect("temporary directory");
        let old = dir.path().join("original.py");
        let new = dir.path().join("replacement.py");
        fs::write(&old, "value = 1").expect("write original");
        fs::copy(&old, &new).expect("copy source");
        let metadata = old.metadata().expect("metadata");
        filetime::set_file_mtime(&new, FileTime::from_last_modification_time(&metadata)).expect("restore mtime");
        let changed = metadata.ctime().max(new.metadata().expect("metadata").ctime());
        let key = |path, age| FileKey::of(path, FileTime::from_unix_time(changed + age, 0)).expect("file key").hash;
        let supported = local_change_times(&File::open(&old).expect("open original"));
        assert_eq!((key(&old, 0) == key(&new, 0), key(&old, 2) != key(&new, 2)), (true, supported));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn procfs_uses_contents_even_after_the_clock_tick() {
        let path = std::path::Path::new("/proc/self/cmdline");
        let changed = path.metadata().expect("procfs metadata").ctime();
        let key = |age| FileKey::of(path, FileTime::from_unix_time(changed + age, 0)).expect("procfs key").hash;
        assert_eq!(key(2), key(0));
    }
}
