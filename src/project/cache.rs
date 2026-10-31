// Ruff's cache, adapted from the `ruff` crate's `cache.rs`: one file per package, keyed by the package root, remembers
// per file, by its modification time and permissions, that it had nothing to report, so a later run skips it. simplint
// also keeps each file's import facts, so the project index does not parse unchanged files, and a clean result counts
// only while the file's settings and its slice of the project index are what they were. With fixes on, a file the fix
// loop left as it was keeps the rules whose fixes the loop withdrew, so a later run reproduces its reports from one
// lint. The files live in a `simplint` directory inside Ruff's cache directory, apart from Ruff's own.
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use filetime::FileTime;
use itertools::Itertools;
use log::error;
use rayon::prelude::*;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::warn_user;
use rustc_hash::FxHashMap;
use tempfile::NamedTempFile;

use crate::cli::diagnostics::{Diagnostics, Mode};
use crate::lint::Linter;
use crate::lint::registry::{Rule, RuleSet};
use crate::project::FileFacts;

// A file's modification time and permissions, hashed as Ruff's `FileCacheKey`.
#[derive(Clone, Copy)]
pub struct FileKey(u64);

impl FileKey {
    pub fn of(path: &Path) -> Option<Self> {
        let metadata = path.metadata().ok()?;
        let mut hasher = CacheKeyHasher::new();
        FileTime::from_last_modification_time(&metadata).cache_key(&mut hasher);
        permissions(&metadata).cache_key(&mut hasher);
        Some(Self(hasher.finish()))
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

// The caches of a run, one per package root, or per directory for files outside a package, as Ruff keeps them.
pub struct Caches(FxHashMap<PathBuf, Cache>);

impl Caches {
    pub fn open<'p>(ruff_cache_dir: &Path, roots: impl Iterator<Item = &'p Path>) -> Self {
        let directory = directory(ruff_cache_dir).join(env!("CARGO_PKG_VERSION"));
        if let Err(error) = init(ruff_cache_dir, &directory) {
            error!("Failed to initialize cache at {}: {error:?}", ruff_cache_dir.display());
        }
        let roots: Vec<&Path> = roots.unique().collect();
        Self(roots.into_par_iter().map(|root| (root.to_path_buf(), Cache::open(&directory, root))).collect())
    }

    pub fn get(&self, root: &Path) -> Option<&Cache> {
        self.0.get(root)
    }

    pub fn persist(self) -> Result<()> {
        self.0.into_par_iter().try_for_each(|(_, cache)| cache.persist())
    }
}

// simplint's directory in Ruff's cache directory.
pub fn directory(ruff_cache_dir: &Path) -> PathBuf {
    ruff_cache_dir.join("simplint")
}

// Ruff's `init`: the tag and the `.gitignore` mark Ruff's cache directory whichever tool creates it first, so an existing
// one stays as it is.
fn init(ruff_cache_dir: &Path, directory: &Path) -> io::Result<()> {
    fs::create_dir_all(directory)?;
    cachedir::ensure_tag(ruff_cache_dir)?;
    match fs::OpenOptions::new().write(true).create_new(true).open(ruff_cache_dir.join(".gitignore")) {
        Ok(mut file) => file.write_all(b"# Automatically created by ruff.\n*\n"),
        Err(error) => (error.kind() == io::ErrorKind::AlreadyExists).then_some(()).ok_or(error),
    }
}

pub struct Cache {
    path: PathBuf,
    package: PackageCache,
    // What this run learned, applied to `package` when the cache is written, which is skipped when there is nothing.
    changes: Mutex<Vec<Change>>,
    now: u64,
}

impl Cache {
    fn open(directory: &Path, package_root: &Path) -> Self {
        let mut hasher = CacheKeyHasher::new();
        package_root.cache_key(&mut hasher);
        let path = directory.join(hasher.finish().to_string());
        let package = match fs::read(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            read => read
                .map_err(anyhow::Error::from)
                .and_then(|bytes| Ok(rkyv::from_bytes::<PackageCache, rkyv::rancor::Error>(&bytes)?))
                .inspect_err(|error| warn_user!("Failed to read cache file `{}`: {error}", path.display()))
                .ok(),
        };
        let package = package
            .filter(|package| package.package_root == package_root)
            .unwrap_or_else(|| PackageCache { package_root: package_root.to_path_buf(), files: FxHashMap::default() });
        let now = SystemTime::UNIX_EPOCH.elapsed().map_or(0, |elapsed| elapsed.as_millis());
        Self { path, package, changes: Mutex::new(Vec::new()), now: u64::try_from(now).unwrap_or(u64::MAX) }
    }

    // The import facts of a file that has not changed since they were cached.
    pub fn facts(&self, path: &Path, key: FileKey) -> Option<&FileFacts> {
        self.get(path, key)?.facts.as_ref()
    }

    pub fn set_facts(&self, path: &Path, key: FileKey, facts: FileFacts) {
        self.update(path, key, ChangeData::Facts(facts));
    }

    // A file whose content, settings and slice of the project index, which `fingerprint` stands for, match a cached
    // clean result is not linted again; a file that lints clean is remembered as such. `lint` gets the rules a cached
    // settled fix loop withdrew, and returns them when the file settles.
    pub fn lint_or_reuse(
        &self,
        path: &Path,
        key: FileKey,
        fingerprint: Fingerprint,
        lint: impl FnOnce(Option<RuleSet>) -> (Diagnostics, Option<RuleSet>),
    ) -> Diagnostics {
        let file = self.get(path, key);
        if file.is_some_and(|file| file.clean == Some(fingerprint.clean)) {
            return Diagnostics::default();
        }
        let known = file
            .and_then(|file| file.settled.as_ref())
            .filter(|settled| fingerprint.settled == Some(settled.fingerprint))
            .map(|settled| RuleSet::from_rules(settled.withdrawn.iter().filter_map(|code| Rule::from_code(code))));
        let (diagnostics, settled) = lint(known);
        if diagnostics.inner.is_empty() && diagnostics.fixed.is_empty() {
            self.update(path, key, ChangeData::Clean(fingerprint.clean));
        } else if let (None, Some(fingerprint), Some(withdrawn)) = (known, fingerprint.settled, settled) {
            self.update(path, key, ChangeData::Settled(fingerprint, withdrawn));
        }
        diagnostics
    }

    fn get(&self, path: &Path, key: FileKey) -> Option<&FileCache> {
        let file = self.package.files.get(path.strip_prefix(&self.package.package_root).ok()?)?;
        if file.key != key.0 {
            return None;
        }
        file.last_seen.store(self.now, Ordering::Relaxed);
        Some(file)
    }

    fn update(&self, path: &Path, key: FileKey, data: ChangeData) {
        if let Ok(relative) = path.strip_prefix(&self.package.package_root) {
            let change = Change { path: relative.to_path_buf(), key: key.0, data };
            self.changes.lock().expect("no worker panics while recording").push(change);
        }
    }

    // Ruff's `persist`: files not seen for 30 days are dropped, and the cache goes to a temporary file renamed over the
    // old one, so a run killed midway or another simplint process never leaves a torn file.
    fn persist(mut self) -> Result<()> {
        const MAX_LAST_SEEN: Duration = Duration::from_hours(720);
        let changes = std::mem::take(self.changes.get_mut().expect("no worker panics while recording"));
        if changes.is_empty() {
            return Ok(());
        }
        let now = self.now;
        let max_last_seen = u64::try_from(MAX_LAST_SEEN.as_millis()).unwrap_or(u64::MAX);
        self.package.files.retain(|_, file| now.saturating_sub(*file.last_seen.get_mut()) <= max_last_seen);
        for Change { path, key, data } in changes {
            let file = self
                .package
                .files
                .entry(path)
                .and_modify(|file| {
                    if file.key != key {
                        *file = FileCache::new(key, now);
                    }
                    file.last_seen.store(now, Ordering::Relaxed);
                })
                .or_insert_with(|| FileCache::new(key, now));
            match data {
                ChangeData::Facts(facts) => file.facts = Some(facts),
                ChangeData::Clean(fingerprint) => file.clean = Some(fingerprint),
                ChangeData::Settled(fingerprint, withdrawn) => {
                    let withdrawn = withdrawn.iter().map(|rule| rule.code().to_owned()).collect();
                    file.settled = Some(Settled { fingerprint, withdrawn });
                }
            }
        }
        let mut temp_file = tempfile_in(self.path.parent().context("cache path has a directory")?)
            .context("Failed to create temporary file")?;
        let serialized =
            rkyv::to_bytes::<rkyv::rancor::Error>(&self.package).context("Failed to serialize cache data")?;
        temp_file.write_all(&serialized).context("Failed to write serialized cache to temporary file.")?;
        let persisted = temp_file.persist(&self.path).map(drop);
        // On Windows another simplint process may hold the cache file open; its run writes the cache instead.
        #[cfg(windows)]
        let persisted = persisted.or_else(|error| {
            if error.error.kind() != io::ErrorKind::PermissionDenied {
                return Err(error);
            }
            warn_user!("Failed to write cache file `{}`: {}", self.path.display(), error.error);
            Ok(())
        });
        persisted.with_context(|| format!("Failed to rename temporary cache file to {}", self.path.display()))?;
        Ok(())
    }
}

// A temporary file with the permissions of a file created the usual way, as `NamedTempFile` defaults to `0o600`.
#[cfg(unix)]
fn tempfile_in(path: &Path) -> io::Result<NamedTempFile> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new().permissions(fs::Permissions::from_mode(0o666)).tempfile_in(path)
}

#[cfg(not(unix))]
fn tempfile_in(path: &Path) -> io::Result<NamedTempFile> {
    tempfile::Builder::new().tempfile_in(path)
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct PackageCache {
    #[rkyv(with = rkyv::with::AsString)]
    package_root: PathBuf,
    // Keyed by the path relative to the package root.
    #[rkyv(with = rkyv::with::MapKV<rkyv::with::AsString, rkyv::with::Identity>)]
    files: FxHashMap<PathBuf, FileCache>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct FileCache {
    key: u64,
    // When a run last saw the file, in milliseconds since the Unix epoch.
    #[rkyv(with = rkyv::with::AtomicLoad<rkyv::with::Relaxed>)]
    last_seen: AtomicU64,
    facts: Option<FileFacts>,
    // The fingerprint the file last linted clean under.
    clean: Option<u64>,
    settled: Option<Settled>,
}

impl FileCache {
    fn new(key: u64, now: u64) -> Self {
        Self { key, last_seen: AtomicU64::new(now), facts: None, clean: None, settled: None }
    }
}

// The fix loop's last run over an unchanged file: the fingerprint it ran under and the codes of the rules whose fixes
// it withdrew. Codes, unlike a rule's position in the rule set, stay put while development builds of a version add
// rules.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Settled {
    fingerprint: u64,
    withdrawn: Vec<String>,
}

struct Change {
    path: PathBuf,
    key: u64,
    data: ChangeData,
}

enum ChangeData {
    Facts(FileFacts),
    Clean(u64),
    Settled(u64, RuleSet),
}

// A file's fingerprint: its settings, where it sits in its package, and its slice of the project index. Most files
// share their project's settings and Ruff's, so those, Ruff's linter settings above all, are hashed once per pair.
#[derive(Default)]
pub struct Fingerprints(Mutex<FxHashMap<(usize, usize), u64>>);

impl Fingerprints {
    pub fn of(&self, linter: &Linter, mode: &Mode) -> Fingerprint {
        let (project, ruff) = (linter.settings.project, linter.ruff.settings);
        let shared = *self
            .0
            .lock()
            .expect("no worker panics while fingerprinting")
            .entry((std::ptr::from_ref(project).addr(), std::ptr::from_ref(ruff).addr()))
            .or_insert_with(|| {
                let mut hasher = CacheKeyHasher::new();
                project.cache_key(&mut hasher);
                ruff.cache_key(&mut hasher);
                hasher.finish()
            });
        let mut hasher = CacheKeyHasher::new();
        shared.cache_key(&mut hasher);
        linter.settings.rules.cache_key(&mut hasher);
        linter.settings.from_ruff.cache_key(&mut hasher);
        // An `__init__.py` added or removed next to the file moves its package root, which keeps the cache file.
        linter.module_path.cache_key(&mut hasher);
        linter.project.cache_key(&mut hasher);
        let clean = hasher.finish();
        // Which fixes apply decides what the fix loop withdraws, and whether it changes the file at all.
        let settled = (!mode.fix.is_generate()).then(|| {
            let mut hasher = CacheKeyHasher::new();
            clean.cache_key(&mut hasher);
            mode.fix.hash(&mut hasher);
            mode.unsafe_fixes.cache_key(&mut hasher);
            mode.noqa.hash(&mut hasher);
            hasher.finish()
        });
        Fingerprint { clean, settled }
    }
}

// What a clean result is valid under, and with fixes on, what a settled fix loop is valid under.
#[derive(Clone, Copy)]
pub struct Fingerprint {
    clean: u64,
    settled: Option<u64>,
}
