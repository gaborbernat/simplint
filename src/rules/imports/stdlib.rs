use super::stdlib_submodules::{LATEST_MINOR, STDLIB_SUBMODULES};
use crate::syntax::{is_stdlib_root, top_level};

pub(super) fn ships_everywhere(target: u8, module: &str) -> bool {
    // Guarding a module that exists on one platform only, or that a minimal or alternative Python build may leave out,
    // is legitimate, so these never count as required.
    const PLATFORM_MODULES: &[&str] = &[
        "_bz2",
        "_ctypes",
        "_curses",
        "_curses_panel",
        "_dbm",
        "_decimal",
        "_gdbm",
        "_hashlib",
        "_lzma",
        "_multiprocessing",
        "_overlapped",
        "_posixshmem",
        "_posixsubprocess",
        "_scproxy",
        "_sqlite3",
        "_ssl",
        "_tkinter",
        "_uuid",
        "_winapi",
        "_wmi",
        "_zstd",
        "bz2",
        "compression.zstd",
        "ctypes",
        "dbm",
        "ensurepip",
        "gzip",
        "idlelib",
        "lzma",
        "mmap",
        "pyexpat",
        "sqlite3",
        "ssl",
        "tkinter",
        "turtle",
        "zlib",
        "asyncio.unix_events",
        "asyncio.windows_events",
        "asyncio.windows_utils",
        "crypt",
        "curses",
        "encodings.mbcs",
        "encodings.oem",
        "fcntl",
        "grp",
        "msvcrt",
        "multiprocessing.popen_forkserver",
        "multiprocessing.popen_spawn_win32",
        "multiprocessing.synchronize",
        "nis",
        "nt",
        "ossaudiodev",
        "posix",
        "pty",
        "pwd",
        "readline",
        "resource",
        "spwd",
        "syslog",
        "termios",
        "tty",
        "winreg",
        "winsound",
        "xml.parsers.expat",
    ];
    // The C accelerators that CPython itself imports under an `ImportError` fallback, so a guard around one is
    // CPython's own pattern: `_bisect` (Lib/bisect.py:111-114), `_collections` (Lib/collections/__init__.py:46-49),
    // `_datetime` (Lib/datetime.py:7-10), `_functools` (Lib/functools.py:225-228), `_heapq` (Lib/heapq.py:596-599),
    // `_json` (Lib/json/scanner.py:4-7), `_operator` (Lib/operator.py:421-424) and `_pickle` (Lib/pickle.py:1970-1971).
    const ACCELERATORS: &[&str] =
        &["_bisect", "_collections", "_datetime", "_functools", "_heapq", "_json", "_operator", "_pickle"];
    let root = top_level(module);
    // A module removed in a later version, such as `distutils` in 3.12, is optional for a project that supports both.
    (target..=LATEST_MINOR).all(|minor| is_stdlib_root(minor, module))
        // CPython builds its `_test*` extensions for its own test suite, which release builds leave out.
        && !root.starts_with("_test")
        && !PLATFORM_MODULES.contains(&root)
        && !PLATFORM_MODULES.contains(&module)
        && !ACCELERATORS.contains(&module)
        && (root == module || is_standard_submodule(target, module))
}

// The top-level table cannot say whether a submodule such as `asyncio.taskgroups` ships on the target, so the
// generated table does. A submodule it lacks is never assumed present.
fn is_standard_submodule(target: u8, module: &str) -> bool {
    STDLIB_SUBMODULES.binary_search_by_key(&module, |submodule| submodule.name).is_ok_and(|index| {
        let submodule = &STDLIB_SUBMODULES[index];
        submodule.added <= target && submodule.removed.is_none()
    })
}
