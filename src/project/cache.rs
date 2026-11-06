// Ruff's cache, adapted from the `ruff` crate's `cache.rs`: one file per package, keyed by the package root, remembers
// per file, by its modification time and permissions, that it had nothing to report, so a later run skips it. Without
// fixes, a file with reports keeps them too. simplint also keeps each file's import facts, so the project index does
// not parse unchanged files, and a clean result counts only while the file's settings and its slice of the project
// index are what they were. With fixes on, a file the fix loop left as it was keeps the rules whose fixes the loop
// withdrew, so a later run reproduces its reports from one lint. The files live in a `simplint` directory inside Ruff's
// cache directory, apart from Ruff's own, under a directory named by `FORMAT`.
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use filetime::FileTime;
use itertools::Itertools;
use log::error;
use rayon::prelude::*;
use rkyv::util::AlignedVec;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_db::diagnostic::{
    Annotation, Diagnostic, DiagnosticId, LintName, SecondaryCode, Severity, Span, SubDiagnosticSeverity,
};
use ruff_linter::warn_user;
use ruff_source_file::{SourceFile, SourceFileBuilder};
use ruff_text_size::{TextRange, TextSize};
use rustc_hash::FxHashMap;
use tempfile::NamedTempFile;

use crate::lint::Linter;
use crate::lint::registry::{Rule, RuleSet};
use crate::project::FileFacts;
use crate::run::{Diagnostics, FixMap, Linting, Mode};

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
    // Makes the cache directory and drops other versions' entries. The documentation key hashes the times of the
    // directories it walks, and the cache directory sits in a project root by default, so creating it while that walk
    // runs would change a key that is then stored, and the next run would miss it.
    pub fn prepare(ruff_cache_dir: &Path) {
        let directory = directory(ruff_cache_dir).join(FORMAT);
        if let Err(error) = init(ruff_cache_dir, &directory) {
            error!("Failed to initialize cache at {}: {error:?}", ruff_cache_dir.display());
        }
        remove_stale(&directory);
    }

    pub fn open<'p>(ruff_cache_dir: &Path, roots: impl Iterator<Item = &'p Path>) -> Self {
        let directory = directory(ruff_cache_dir).join(FORMAT);
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

// The version and a hash of the cached types' source, which `build.rs` computes, so a development build that changes
// them does not read caches an earlier build of the same version wrote.
const FORMAT: &str = concat!(env!("CARGO_PKG_VERSION"), "-", env!("SIMPLINT_CACHE_FORMAT"));

// How long a cached file, or the directory of another simplint build, stays without a run that uses it, as in Ruff.
const MAX_LAST_SEEN: Duration = Duration::from_hours(720);

// The file that keeps what the documentation under a project root names, among the package caches of this build and
// named as they are, by a key in decimal.
pub fn documentation_file(ruff_cache_dir: &Path, project_root: &Path) -> PathBuf {
    let mut hasher = CacheKeyHasher::new();
    "documentation".cache_key(&mut hasher);
    project_root.cache_key(&mut hasher);
    directory(ruff_cache_dir).join(FORMAT).join(hasher.finish().to_string())
}

// Writes a cache file through a temporary file next to it, so a reader never sees it cut short. The directory is made
// first, as the run may write before it has opened the package caches.
pub fn write_atomically(path: &Path, bytes: &[u8]) -> Result<()> {
    let directory = path.parent().context("a cache file sits in a directory")?;
    fs::create_dir_all(directory)?;
    let mut temporary = tempfile_in(directory)?;
    temporary.write_all(bytes)?;
    temporary.persist(path)?;
    Ok(())
}

// simplint's directory in Ruff's cache directory.
pub fn directory(ruff_cache_dir: &Path) -> PathBuf {
    ruff_cache_dir.join("simplint")
}

// Removes the directories other simplint builds wrote that no run has written to for `MAX_LAST_SEEN`. A directory
// another version still writes to stays, as when pre-commit pins one version and the command line runs another. Its
// newest file tells its age, since Windows does not move a directory's modification time when a file in it changes; an
// empty one, as another run creates before it writes its cache, goes by its own.
fn remove_stale(current: &Path) {
    let Some(Ok(siblings)) = current.parent().map(fs::read_dir) else {
        return;
    };
    let now = SystemTime::now();
    for entry in siblings.filter_map(Result::ok) {
        let sibling = entry.path();
        if sibling == current || !is_build_directory(&entry) {
            continue;
        }
        let newest = fs::read_dir(&sibling)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok()?.metadata().ok()?.modified().ok())
            .max()
            .or_else(|| sibling.metadata().ok()?.modified().ok());
        let stale = newest.is_none_or(|modified| now.duration_since(modified).is_ok_and(|age| age > MAX_LAST_SEEN));
        // Removal is best effort: a directory it cannot remove, as one another process has open, waits for a later run.
        if stale {
            let _ = fs::remove_dir_all(&sibling);
        }
    }
}

// Whether an entry is another build's cache directory, the only kind `remove_stale` removes: a real directory, not a
// link, named as `FORMAT` names one and holding nothing but cache files. Anything else in the `simplint` directory, as
// a directory a user put there, stays.
fn is_build_directory(entry: &fs::DirEntry) -> bool {
    entry.file_name().to_str().is_some_and(is_build_name)
        && entry.file_type().is_ok_and(|file_type| file_type.is_dir())
        && holds_only_cache_files(&entry.path())
}

// Whether a name has the shape `FORMAT` gives: a version, then the 16 hex digits of the hash `build.rs` computes.
fn is_build_name(name: &str) -> bool {
    name.rsplit_once('-').is_some_and(|(version, hash)| {
        !version.is_empty()
            && version.bytes().all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'+'))
            && hash.len() == 16
            && hash.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    })
}

// Whether a directory holds only cache files, which `persist` names by their package's key, in decimal.
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
        // rkyv reads the archive in place, which needs its bytes aligned.
        let read = fs::File::open(&path).and_then(|mut file| {
            let mut bytes = AlignedVec::<16>::new();
            bytes.extend_from_reader(&mut file).map(|_| bytes)
        });
        let package = match read {
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            read => read
                .map_err(anyhow::Error::from)
                .and_then(|bytes| PackageCache::read(&bytes))
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
        self.update(path, key, ChangeData::Facts(Box::new(facts)));
    }

    // A file whose content, settings and slice of the project index, which `fingerprint` stands for, match a cached
    // clean result is not linted again; a file that lints clean is remembered as such. Without fixes, a file whose
    // reports are cached under the same fingerprint gets them back, as they were reported. `lint` gets the rules a
    // cached settled fix loop withdrew, and returns them when the file settles.
    pub fn lint_or_reuse(
        &self,
        path: &Path,
        key: FileKey,
        fingerprint: Fingerprint,
        lint: impl FnOnce(Option<RuleSet>) -> Linting,
    ) -> Diagnostics {
        let file = self.get(path, key);
        if file.is_some_and(|file| file.clean == Some(fingerprint.clean)) {
            return Diagnostics::default();
        }
        if fingerprint.settled.is_none() {
            let findings =
                file.and_then(|file| file.findings.as_ref()).filter(|found| found.fingerprint == fingerprint.clean);
            if let Some(diagnostics) = findings.and_then(|found| found.diagnostics(path)) {
                return diagnostics;
            }
        }
        let known = file
            .and_then(|file| file.settled.as_ref())
            .filter(|settled| fingerprint.settled == Some(settled.fingerprint))
            .map(|settled| RuleSet::from_rules(settled.withdrawn.iter().filter_map(|code| Rule::from_code(code))));
        let Linting { diagnostics, settled, noqa_rows } = lint(known);
        if diagnostics.inner.is_empty() && diagnostics.fixed.is_empty() {
            self.update(path, key, ChangeData::Clean(fingerprint.clean));
        } else if let (None, Some(fingerprint), Some(withdrawn)) = (known, fingerprint.settled, settled) {
            self.update(path, key, ChangeData::Settled(fingerprint, withdrawn));
        } else if fingerprint.settled.is_none()
            && let Some(reports) = Report::all(&diagnostics.inner, &noqa_rows)
        {
            self.update(path, key, ChangeData::Findings(fingerprint.clean, reports));
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
                ChangeData::Facts(facts) => file.facts = Some(*facts),
                ChangeData::Clean(fingerprint) => {
                    file.clean = Some(fingerprint);
                    file.findings = None;
                }
                ChangeData::Findings(fingerprint, reports) => {
                    file.findings = Some(Findings { fingerprint, reports });
                    file.clean = None;
                }
                ChangeData::Settled(fingerprint, withdrawn) => {
                    let withdrawn = withdrawn.iter().map(|rule| rule.code().to_owned()).collect();
                    file.settled = Some(Settled { fingerprint, withdrawn });
                }
            }
        }
        let directory = self.path.parent().context("cache path has a directory")?;
        // Another simplint build may have removed the directory as stale since this run opened it.
        fs::create_dir_all(directory).context("Failed to create cache directory")?;
        let mut temp_file = tempfile_in(directory).context("Failed to create temporary file")?;
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

impl PackageCache {
    // The archive checked as a whole, then its files read back on all threads: a project's top package holds most of
    // its files, and reading them one after another would leave the run waiting on that one cache.
    fn read(bytes: &[u8]) -> Result<Self> {
        let archived = rkyv::access::<ArchivedPackageCache, rkyv::rancor::Error>(bytes)?;
        let entries: Vec<_> = archived.files.iter().collect();
        let files = entries
            .into_par_iter()
            .map(|(path, file)| {
                Ok((PathBuf::from(path.as_str()), rkyv::deserialize::<FileCache, rkyv::rancor::Error>(file)?))
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
    facts: Option<FileFacts>,
    // The fingerprint the file last linted clean under.
    clean: Option<u64>,
    settled: Option<Settled>,
    // The reports the file last had without fixes, under their fingerprint.
    findings: Option<Findings>,
}

impl FileCache {
    fn new(key: u64, now: u64) -> Self {
        Self { key, last_seen: AtomicU64::new(now), facts: None, clean: None, settled: None, findings: None }
    }
}

// The reports a file had without fixes, under the fingerprint they were reported under.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Findings {
    fingerprint: u64,
    reports: Vec<Report>,
}

impl Findings {
    fn diagnostics(&self, path: &Path) -> Option<Diagnostics> {
        let source = fs::read_to_string(path).ok()?;
        let file = SourceFileBuilder::new(path.to_string_lossy(), source.as_str()).finish();
        let inner = self.reports.iter().map(|report| report.diagnostic(&file)).collect::<Option<_>>()?;
        Some(Diagnostics { inner, fixed: FixMap::default(), diffs: Vec::new() })
    }
}

// The rule of a code, looked up in a map since a warm run looks up every cached report.
fn rule_by_code(code: &str) -> Option<Rule> {
    static RULES: OnceLock<FxHashMap<&'static str, Rule>> = OnceLock::new();
    RULES.get_or_init(|| Rule::ALL.iter().map(|rule| (rule.code(), *rule)).collect()).get(code).copied()
}

// A diagnostic a rule reported, with the fields the output shows. `Report::of` takes only a diagnostic the checker
// builds, so `Report::diagnostic` gives back the same one.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Report {
    code: String,
    message: String,
    start: u32,
    end: u32,
    // The line of the suppression comment, as `Suppressions::locate` sets it.
    noqa_offset: u32,
    help: Vec<String>,
    // The fix as serde writes it, which keeps each of its fields.
    fix: Option<String>,
}

impl Report {
    // All of the reports, or `None` when a diagnostic is one `of` cannot store, such as a parse error.
    fn all(diagnostics: &[Diagnostic], noqa_rows: &[TextSize]) -> Option<Vec<Self>> {
        let reports: Option<Vec<Self>> =
            diagnostics.iter().zip(noqa_rows).map(|(diagnostic, row)| Self::of(diagnostic, *row)).collect();
        reports.filter(|reports| reports.len() == diagnostics.len())
    }

    // The report of a rule's diagnostic. Only what the checker sets is stored: a diagnostic with more than its one
    // primary annotation, a parent, a severity other than error, or a sub-diagnostic other than a help note is not.
    fn of(diagnostic: &Diagnostic, noqa_row: TextSize) -> Option<Self> {
        let rule = rule_by_code(diagnostic.secondary_code()?.as_str())?;
        let url = rule.url();
        let primary = diagnostic.annotations().first()?;
        let range = primary.get_span().range()?;
        let help: Vec<String> = diagnostic
            .sub_diagnostics()
            .iter()
            .map(|sub| {
                let help = sub.severity() == SubDiagnosticSeverity::Help && sub.annotations().is_empty();
                help.then(|| sub.headline_message().to_owned())
            })
            .collect::<Option<_>>()?;
        let plain = diagnostic.annotations().len() == 1
            && primary.is_primary()
            && primary.get_message().is_none()
            && diagnostic.primary_tags().is_none_or(<[_]>::is_empty)
            && diagnostic.id() == DiagnosticId::Lint(LintName::of(rule.name()))
            && diagnostic.severity() == Severity::Error
            && diagnostic.documentation_url() == Some(url.as_str())
            && diagnostic.parent().is_none();
        let fix = diagnostic.fix().map(serde_json::to_string).transpose().ok()?;
        plain.then(|| Self {
            code: rule.code().to_owned(),
            message: diagnostic.headline_message().to_owned(),
            start: range.start().to_u32(),
            end: range.end().to_u32(),
            noqa_offset: noqa_row.to_u32(),
            help,
            fix,
        })
    }

    // The diagnostic `of` took the report from, given the file it reports on.
    fn diagnostic(&self, file: &SourceFile) -> Option<Diagnostic> {
        let rule = rule_by_code(&self.code)?;
        let range = TextRange::new(TextSize::new(self.start), TextSize::new(self.end));
        let mut diagnostic =
            Diagnostic::new(DiagnosticId::Lint(LintName::of(rule.name())), Severity::Error, self.message.as_str());
        diagnostic.annotate(Annotation::primary(Span::from(file.clone()).with_range(range)));
        diagnostic.set_secondary_code(SecondaryCode::new(rule.code().to_owned()));
        diagnostic.set_documentation_url(Some(rule.url()));
        for help in &self.help {
            diagnostic.help(help.as_str());
        }
        if let Some(fix) = &self.fix {
            diagnostic.set_fix(serde_json::from_str(fix).ok()?);
        }
        diagnostic.set_noqa_offset(TextSize::new(self.noqa_offset));
        Some(diagnostic)
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
    Facts(Box<FileFacts>),
    Clean(u64),
    Findings(u64, Vec<Report>),
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
        let rules = linter.settings.rules;
        rules.cache_key(&mut hasher);
        linter.settings.from_ruff.cache_key(&mut hasher);
        // An `__init__.py` added or removed next to the file moves its package root, which keeps the cache file. In a
        // namespace package that root stays, so the rules that read the file's `__init__.py` itself key it too.
        linter.module_path.cache_key(&mut hasher);
        if rules.overlaps(RuleSet::sibling_init()) {
            linter.sibling_init.cache_key(&mut hasher);
        }
        // Only a rule that reads other files depends on the index of the run, so the others keep their results when
        // another file changes.
        if rules.overlaps(RuleSet::cross_file()) {
            linter.project.cache_key(&mut hasher);
        }
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
