use std::env;
use std::process::Command;

const CHILD: &str = "SIMPLINT_TEST_LIMITED";

#[derive(Clone, Copy)]
pub enum Limit {
    // No file can grow, so a write fails as on a full disk.
    FileSize,
    // No file descriptor is left, so opening anything fails.
    #[cfg(target_os = "linux")]
    OpenFiles,
}

// Runs `body` on what `prepare` made under `limit`. A limit is process-wide, so the test `name` runs again in a child
// process that alone has it; the limit is lifted before the child exits, so the coverage profile it writes then still
// fits.
pub fn limited<T>(name: &str, limit: Limit, prepare: impl FnOnce() -> T, body: impl FnOnce(T)) {
    let test = name.split_once("::").map_or(name, |(_, test)| test);
    if env::var_os(CHILD).is_none() {
        let output = Command::new(env::current_exe().expect("the test binary has a path"))
            .args([test, "--exact", "--test-threads", "1"])
            .env(CHILD, "1")
            .output()
            .expect("the test binary runs");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.contains("test result: ok. 1 passed"), "{test} did not pass in the child process:\n{stdout}");
        return;
    }
    let prepared = prepare();
    let (resource, current) = match limit {
        Limit::FileSize => (libc::RLIMIT_FSIZE, 0),
        // Descriptors are handed out lowest first, so a limit at the lowest free one leaves none to hand out.
        #[cfg(target_os = "linux")]
        Limit::OpenFiles => {
            use std::os::fd::AsRawFd;

            let probe = std::fs::File::open("/dev/null").expect("a descriptor is free");
            (libc::RLIMIT_NOFILE, libc::rlim_t::try_from(probe.as_raw_fd()).expect("a descriptor is not negative"))
        }
    };
    let mut old = libc::rlimit { rlim_cur: 0, rlim_max: 0 };
    // SAFETY: plain system calls on values on the stack. Ignoring `SIGXFSZ` makes a write past the file size limit fail
    // with `EFBIG` rather than end the process.
    unsafe {
        libc::signal(libc::SIGXFSZ, libc::SIG_IGN);
        assert_eq!(libc::getrlimit(resource, &raw mut old), 0);
        assert_eq!(libc::setrlimit(resource, &libc::rlimit { rlim_cur: current, rlim_max: old.rlim_max }), 0);
    }
    body(prepared);
    // SAFETY: as above; the soft limit goes back to its old value, which the unchanged hard limit allows.
    unsafe {
        assert_eq!(libc::setrlimit(resource, &raw const old), 0);
    }
}
