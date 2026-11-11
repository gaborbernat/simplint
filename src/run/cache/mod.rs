// Ruff's cache, adapted from the `ruff` crate's `cache.rs`: one file per package, keyed by the package root, remembers
// per file, by its modification time, permissions and size, that it had nothing to report, so a later run skips it.
// Without fixes, a file with reports keeps them too. simplint also keeps each file's import facts, so the project index
// does not parse unchanged files, and a clean result counts only while the file's settings and its slice of the project
// index are what they were. With fixes on, a file the fix loop left as it was keeps the rules whose fixes the loop
// withdrew, so a later run reproduces its reports from one lint.
mod fingerprint;
mod rendered;
mod report;

use std::fs;
use std::hash::Hasher;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::SystemTime;

use anyhow::{Context, Result};
use itertools::Itertools;
use log::error;
use rayon::prelude::*;
use rkyv::util::AlignedVec;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::warn_user;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::fix::Withdrawn;
use crate::project::cache::{FileKey, MAX_LAST_SEEN, build_directory, read_aligned, tempfile_in, write_atomically};
use crate::project::{Ending, FileFacts};
use crate::run::{Diagnostics, Linting, warn_unparsed};

use self::fingerprint::Fingerprint;
use self::rendered::{Rendered, RenderedPackage, read_rendered, text};
use self::report::{ArchivedFindings, Findings, Report, Settled, withdrawn};

pub(super) use fingerprint::{FileIdentity, Fingerprints};
pub(crate) use rendered::{DisplayKey, FullDisplay};

// The caches of a run, one per package root, or per directory for files outside a package, as Ruff keeps them. Their
// bytes are read up front and checked once by `views`, a package per thread, and read in place.
pub(super) struct Caches(FxHashMap<PathBuf, Stored>);

struct Stored {
    path: PathBuf,
    bytes: Option<AlignedVec<16>>,
    // The package's `full` text, kept beside the cache file.
    rendered: PathBuf,
}

impl Caches {
    pub(super) fn open<'p>(ruff_cache_dir: &Path, roots: impl Iterator<Item = &'p Path>) -> Self {
        let directory = build_directory(ruff_cache_dir);
        let roots: Vec<&Path> = roots.unique().collect();
        Self(roots.into_par_iter().map(|root| (root.to_path_buf(), Stored::read(&directory, root))).collect())
    }

    pub(super) fn views(&self, run: u64) -> FxHashMap<&Path, Cache<'_>> {
        self.0.par_iter().map(|(root, stored)| (root.as_path(), Cache::new(root, stored, run))).collect()
    }
}

impl Stored {
    fn read(directory: &Path, package_root: &Path) -> Self {
        let mut hasher = CacheKeyHasher::new();
        package_root.cache_key(&mut hasher);
        let path = directory.join(hasher.finish().to_string());
        // rkyv reads the archive in place, which needs its bytes aligned.
        let bytes = match read_aligned(&path) {
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => {
                warn_user!("Failed to read cache file `{}`: {error}", path.display());
                None
            }
            Ok(bytes) => Some(bytes),
        };
        let mut hasher = CacheKeyHasher::new();
        "rendered".cache_key(&mut hasher);
        package_root.cache_key(&mut hasher);
        let rendered = directory.join(hasher.finish().to_string());
        Self { path, bytes, rendered }
    }
}

pub(super) struct Cache<'a> {
    path: PathBuf,
    package_root: &'a Path,
    // The package's archive, when the cache file holds one for this root and reads back.
    archived: Option<&'a ArchivedPackageCache>,
    // The files whose key matched this run, so their `last_seen` is set when the cache is written.
    seen: Mutex<FxHashSet<String>>,
    // What this run learned, applied to the package when the cache is written, which is skipped when there is nothing.
    changes: Mutex<Vec<Change>>,
    now: u64,
    // The digest of the index this run builds, which the reports it stores are valid under.
    run: u64,
    // The package's `full` text, read the first time a run shows `full` output, and the file it is written to.
    rendered: OnceLock<FxHashMap<String, Rendered>>,
    rendered_path: PathBuf,
    // The text this run rendered, by the path of its file relative to the package root.
    texts: Mutex<Vec<(String, Rendered)>>,
}

impl<'a> Cache<'a> {
    fn new(package_root: &'a Path, stored: &'a Stored, run: u64) -> Self {
        let archived = stored.bytes.as_ref().and_then(|bytes| {
            rkyv::access::<ArchivedPackageCache, rkyv::rancor::Error>(bytes)
                .inspect_err(|error| warn_user!("Failed to read cache file `{}`: {error}", stored.path.display()))
                .ok()
                .filter(|archived| Path::new(archived.package_root.as_str()) == package_root)
        });
        Self {
            path: stored.path.clone(),
            package_root,
            archived,
            seen: Mutex::new(FxHashSet::default()),
            changes: Mutex::new(Vec::new()),
            now: u64::try_from(SystemTime::UNIX_EPOCH.elapsed().map_or(0, |elapsed| elapsed.as_millis()))
                .unwrap_or(u64::MAX),
            run,
            rendered: OnceLock::new(),
            rendered_path: stored.rendered.clone(),
            texts: Mutex::new(Vec::new()),
        }
    }

    // The import facts of a file that has not changed since they were cached, unless their ranges do not fit the file.
    pub(super) fn facts(&self, path: &Path, key: FileKey) -> Option<FileFacts> {
        let stored = self.get(path, key)?.facts.as_ref()?;
        // rkyv reads the archive in place, which needs its bytes aligned.
        let mut bytes = AlignedVec::<16>::with_capacity(stored.len());
        bytes.extend_from_slice(stored);
        rkyv::from_bytes::<FileFacts, rkyv::rancor::Error>(&bytes)
            .inspect_err(|error| error!("Failed to read the facts of {}: {error}", path.display()))
            .ok()
            .filter(|facts| facts.fits(key.length))
    }

    pub(super) fn set_facts(&self, path: &Path, key: FileKey, facts: &FileFacts) {
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(facts).expect("facts archive into memory");
        self.update(path, key, ChangeData::Facts(bytes.to_vec()));
    }

    // A file whose content, settings and slice of the project index, which `fingerprint` stands for, match a cached
    // clean result is not linted again; a file that lints clean is remembered as such. Without fixes, a file whose
    // reports are cached under the same fingerprint gets them back, as they were reported. `lint` gets the rules a
    // cached settled fix loop withdrew, and returns them when the file settles.
    pub(super) fn lint_or_reuse(
        &self,
        path: &Path,
        key: FileKey,
        fingerprint: Fingerprint,
        full: Option<&FullDisplay>,
        lint: impl FnOnce(Option<Withdrawn>) -> Linting,
    ) -> Diagnostics {
        let file = self.get(path, key);
        let index = IndexKey { run: self.run, base: fingerprint.base };
        // A result reused under the full fingerprint holds under this run's index too, so it is marked as such: a
        // later run with this index then reuses it without building the index, as it does results linted now.
        let relabel = |file: &ArchivedFileCache| {
            let current = file.index.as_ref().is_some_and(|stored| stored.is(index.run, index.base));
            if !current {
                self.update(path, key, ChangeData::Index(index));
            }
        };
        if let Some(file) = file
            && file.clean.as_ref().is_some_and(|clean| clean.to_native() == fingerprint.clean)
        {
            relabel(file);
            return clean_result(file, path);
        }
        if fingerprint.settled.is_none()
            && let Some(file) = file
            && let Some(found) =
                file.findings.as_ref().filter(|found| found.fingerprint.to_native() == fingerprint.clean)
            && let Some(diagnostics) = self.stored_findings(path, key, found, full)
        {
            relabel(file);
            return diagnostics;
        }
        let known = file
            .and_then(|file| file.settled.as_ref())
            .filter(|settled| fingerprint.settled == Some(settled.fingerprint.to_native()))
            .map(withdrawn);
        let fresh = known.is_none();
        let Linting { mut diagnostics, settled, noqa_rows, cacheable, unparsed } = lint(known);
        if !cacheable {
            return diagnostics;
        }
        // Text is kept only for a file the cache already held reports for, so a first run that has nothing to reuse
        // renders the output as it did before, and the next run reuses the text.
        let rendered = full
            .filter(|_| file.is_some_and(|file| file.findings.is_some()) && !diagnostics.inner.is_empty())
            .map(|full| text(full, &path.to_string_lossy(), &mut diagnostics, key.hash, fingerprint.clean));
        if diagnostics.inner.is_empty() && diagnostics.fixed.is_empty() {
            self.update(path, key, ChangeData::Clean(fingerprint.clean, index, unparsed));
        } else if let (true, Some(fingerprint), Some(withdrawn)) = (fresh, fingerprint.settled, settled) {
            self.update(path, key, ChangeData::Settled(fingerprint, withdrawn));
        } else if fingerprint.settled.is_none()
            && let Some(reports) = Report::all(&diagnostics.inner, &noqa_rows)
        {
            self.update(path, key, ChangeData::Findings(fingerprint.clean, reports, index));
            if let Some(rendered) = rendered {
                self.store_text(path, rendered);
            }
        }
        diagnostics
    }

    // The clean result or reports a file's entry holds, when the run that stored them built the same index this run
    // would and the file has the same base settings. Then the index need not be built to answer for the file.
    pub(super) fn reused(
        &self,
        path: &Path,
        key: FileKey,
        base: u64,
        full: Option<&FullDisplay>,
    ) -> Option<Diagnostics> {
        let file = self.get(path, key)?;
        let index = file.index.as_ref()?;
        if !index.is(self.run, base) {
            return None;
        }
        if file.clean.is_some() {
            return Some(clean_result(file, path));
        }
        self.stored_findings(path, key, file.findings.as_ref()?, full)
    }

    fn stored_findings(
        &self,
        path: &Path,
        key: FileKey,
        found: &ArchivedFindings,
        full: Option<&FullDisplay>,
    ) -> Option<Diagnostics> {
        let mut diagnostics = Findings::diagnostics(found, path)?;
        if let Some(full) = full {
            self.stored_text(path, key, found.fingerprint.to_native(), full, &mut diagnostics);
        }
        Some(diagnostics)
    }

    // The `full` text of a file's reports under `fingerprint`: the text the package stores for the file when the file's
    // key, the fingerprint and the display settings all match, otherwise the text rendered now, which the package keeps
    // for the next run.
    fn stored_text(
        &self,
        path: &Path,
        key: FileKey,
        fingerprint: u64,
        full: &FullDisplay,
        diagnostics: &mut Diagnostics,
    ) {
        let name = path.to_string_lossy().into_owned();
        let stored = self.relative(path).and_then(|relative| self.stored_rendered().get(relative));
        match stored {
            Some(stored)
                if stored.file == key.hash && stored.findings == fingerprint && stored.key == full.key(&name) =>
            {
                diagnostics.rendered.insert(name, stored.text.clone());
            }
            _ => self.store_text(path, text(full, &name, diagnostics, key.hash, fingerprint)),
        }
    }

    // The package's stored `full` text, read the first time a run needs it, so other output never reads it.
    fn stored_rendered(&self) -> &FxHashMap<String, Rendered> {
        self.rendered.get_or_init(|| read_rendered(&self.rendered_path))
    }

    fn store_text(&self, path: &Path, rendered: Rendered) {
        if let Some(relative) = self.relative(path) {
            recorded(&self.texts).push((relative.to_owned(), rendered));
        }
    }

    fn get(&self, path: &Path, key: FileKey) -> Option<&'a ArchivedFileCache> {
        let relative = self.relative(path)?;
        let file = self.archived?.files.get(relative)?;
        if file.key.to_native() != key.hash {
            return None;
        }
        recorded(&self.seen).insert(relative.to_owned());
        Some(file)
    }

    fn update(&self, path: &Path, key: FileKey, data: ChangeData) {
        if let Some(relative) = self.relative(path) {
            let change = Change { path: relative.to_owned(), key: key.hash, data };
            recorded(&self.changes).push(change);
        }
    }

    fn relative<'b>(&self, path: &'b Path) -> Option<&'b str> {
        path.strip_prefix(self.package_root).ok()?.to_str()
    }

    // Ruff's `persist`: files not seen for 30 days are dropped, and the cache goes to a temporary file renamed over the
    // old one, so a run killed midway or another simplint process never leaves a torn file. Nothing is written when
    // the run changed nothing, so the archive is only deserialized when there is something to write.
    pub(super) fn persist(mut self, ending: Ending) -> Result<()> {
        let result = self.write();
        ending.end(self);
        result
    }

    fn write(&mut self) -> Result<()> {
        let changes = std::mem::take(&mut *recorded(&self.changes));
        let texts = std::mem::take(&mut *recorded(&self.texts));
        if changes.is_empty() && texts.is_empty() {
            return Ok(());
        }
        let now = self.now;
        let mut package = match self.archived {
            Some(archived) => PackageCache::read(archived)?,
            None => PackageCache { package_root: self.package_root.to_path_buf(), files: FxHashMap::default() },
        };
        for relative in recorded(&self.seen).drain() {
            if let Some(file) = package.files.get_mut(&relative) {
                file.last_seen.store(now, Ordering::Relaxed);
            }
        }
        let max_last_seen = u64::try_from(MAX_LAST_SEEN.as_millis()).unwrap_or(u64::MAX);
        package.files.retain(|_, file| now.saturating_sub(*file.last_seen.get_mut()) <= max_last_seen);
        for Change { path, key, data } in changes {
            let file = package
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
                ChangeData::Clean(fingerprint, index, unparsed) => {
                    file.clean = Some(fingerprint);
                    file.unparsed = unparsed;
                    file.findings = None;
                    file.index = Some(index);
                }
                ChangeData::Findings(fingerprint, reports, index) => {
                    file.findings = Some(Findings { fingerprint, reports });
                    file.clean = None;
                    file.index = Some(index);
                }
                ChangeData::Settled(fingerprint, withdrawn) => {
                    file.settled = Some(Settled::new(fingerprint, &withdrawn));
                }
                ChangeData::Index(index) => file.index = Some(index),
            }
        }
        let directory = self.path.parent().context("cache path has a directory")?;
        // Another simplint build may have removed the directory as stale since this run opened it.
        fs::create_dir_all(directory).context("Failed to create cache directory")?;
        let mut temp_file = tempfile_in(directory).context("Failed to create temporary file")?;
        let serialized = rkyv::to_bytes::<rkyv::rancor::Error>(&package).context("Failed to serialize cache data")?;
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
        if !texts.is_empty() {
            let mut files = self.rendered.take().unwrap_or_else(|| read_rendered(&self.rendered_path));
            files.extend(texts);
            // The text of a file that left the package goes with it.
            files.retain(|relative, _| package.files.contains_key(relative));
            let serialized = rkyv::to_bytes::<rkyv::rancor::Error>(&RenderedPackage { files })
                .context("Failed to serialize rendered text")?;
            write_atomically(&self.rendered_path, &serialized)?;
        }
        Ok(())
    }
}

// The result of a file whose clean fingerprint matches: nothing to report, and for a file that does not parse, its
// warning again.
fn clean_result(file: &ArchivedFileCache, path: &Path) -> Diagnostics {
    if file.unparsed {
        warn_unparsed(path);
    }
    Diagnostics::default()
}

fn recorded<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().expect("no worker panics while recording")
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct PackageCache {
    #[rkyv(with = rkyv::with::AsString)]
    package_root: PathBuf,
    // Keyed by the path relative to the package root. A `String` key, as an archived map hashes its keys as they are
    // archived, so a lookup by `str` finds them.
    files: FxHashMap<String, FileCache>,
}

impl PackageCache {
    // The archive's files read back on all threads: a project's top package holds most of its files, and reading them
    // one after another would leave the run waiting on that one cache.
    fn read(archived: &ArchivedPackageCache) -> Result<Self> {
        let entries: Vec<_> = archived.files.iter().collect();
        let files = entries
            .into_par_iter()
            .map(|(path, file)| {
                Ok((path.as_str().to_owned(), rkyv::deserialize::<FileCache, rkyv::rancor::Error>(file)?))
            })
            .collect::<Result<_>>()?;
        Ok(Self { package_root: PathBuf::from(archived.package_root.as_str()), files })
    }
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct FileCache {
    key: u64,
    // When a run last saw the file, in milliseconds since the Unix epoch.
    #[rkyv(with = rkyv::with::AtomicLoad<rkyv::with::Relaxed>)]
    last_seen: AtomicU64,
    // The file's `FileFacts`, archived on their own: writing the cache reads back every file it keeps, and bytes copy
    // where the facts' strings and lists would each be rebuilt.
    facts: Option<Vec<u8>>,
    // The fingerprint the file last linted clean under. A file that does not parse also lints clean, and `unparsed`
    // marks it, since its warning shows on every run.
    clean: Option<u64>,
    unparsed: bool,
    settled: Option<Settled>,
    findings: Option<Findings>,
    // The index and base settings the clean result or the reports were stored under.
    index: Option<IndexKey>,
}

impl FileCache {
    fn new(key: u64, now: u64) -> Self {
        Self {
            key,
            last_seen: AtomicU64::new(now),
            facts: None,
            clean: None,
            unparsed: false,
            settled: None,
            findings: None,
            index: None,
        }
    }
}

#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct IndexKey {
    run: u64,
    base: u64,
}

impl ArchivedIndexKey {
    fn is(&self, run: u64, base: u64) -> bool {
        self.run.to_native() == run && self.base.to_native() == base
    }
}

struct Change {
    path: String,
    key: u64,
    data: ChangeData,
}

enum ChangeData {
    Facts(Vec<u8>),
    Clean(u64, IndexKey, bool),
    Findings(u64, Vec<Report>, IndexKey),
    Settled(u64, Withdrawn),
    Index(IndexKey),
}
