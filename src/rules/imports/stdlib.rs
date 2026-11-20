use super::stdlib_submodules::{LATEST_MINOR, STDLIB_SUBMODULES};
use crate::syntax::{is_stdlib_root, top_level};

// A compatibility branch can import a module from a Python version other than the target.
pub(super) fn ships_on_some_version(module: &str) -> bool {
    (7..=LATEST_MINOR).any(|minor| is_stdlib_root(minor, module))
}

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
        "_lsprof",
        "_lzma",
        "_multiprocessing",
        "_overlapped",
        "_posixshmem",
        "_posixsubprocess",
        "_scproxy",
        "_socket",
        "_sqlite3",
        "_ssl",
        "_tkinter",
        "_tracemalloc",
        "_uuid",
        "_winapi",
        "_wmi",
        "_zstd",
        "bz2",
        "cProfile",
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
        "tracemalloc",
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
    // CPython has pure Python fallbacks for these accelerators, so the same import guard is valid in user code.
    const ACCELERATORS: &[&str] =
        &["_bisect", "_collections", "_datetime", "_functools", "_heapq", "_json", "_operator", "_pickle"];
    let root = top_level(module);
    // A module removed in a later version, such as `distutils` in 3.12, is optional for a project that supports both.
    (target..=LATEST_MINOR).all(|minor| is_stdlib_root(minor, module))
        // Python builds can omit the `_test*` extensions.
        && !root.starts_with("_test")
        && !PLATFORM_MODULES.contains(&root)
        && !PLATFORM_MODULES.contains(&module)
        && !ACCELERATORS.contains(&module)
        && (root == module || is_standard_submodule(target, module))
}

// Finding a package in the standard library does not establish which versions include a particular submodule.
fn is_standard_submodule(target: u8, module: &str) -> bool {
    STDLIB_SUBMODULES.binary_search_by_key(&module, |submodule| submodule.name).is_ok_and(|index| {
        let submodule = &STDLIB_SUBMODULES[index];
        submodule.added <= target && submodule.removed.is_none()
    })
}
