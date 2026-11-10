// Ruff's cache, adapted from the `ruff` crate's `cache.rs`: one file per package, keyed by the package root, remembers
// per file, by its modification time, permissions and size, that it had nothing to report, so a later run skips it. Without
// fixes, a file with reports keeps them too. simplint also keeps each file's import facts, so the project index does
// not parse unchanged files, and a clean result counts only while the file's settings and its slice of the project
// index are what they were. With fixes on, a file the fix loop left as it was keeps the rules whose fixes the loop
// withdrew, so a later run reproduces its reports from one lint. The files live where `project::cache` places them.
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use std::time::SystemTime;

use anyhow::{Context, Result};
use itertools::Itertools;
use log::error;
use rayon::prelude::*;
use rkyv::util::AlignedVec;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_db::diagnostic::{
    Annotation, Diagnostic, DiagnosticId, DisplayDiagnosticConfig, LintName, SecondaryCode, Severity, Span,
    SubDiagnosticSeverity, UnifiedFile,
};
use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
use ruff_linter::message::{RenderContext, render_diagnostics};
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::types::OutputFormat;
use ruff_linter::{IOError, Violation, warn_user};
use ruff_source_file::{SourceFile, SourceFileBuilder};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

use ruff_linter::settings::LinterSettings;

use crate::fix::Withdrawn;
use crate::lint::Linter;
use crate::lint::checker::source_file_at;
use crate::lint::registry::{Rule, RuleSet};
use crate::project::cache::{FileKey, MAX_LAST_SEEN, build_directory, read_aligned, tempfile_in, write_atomically};
use crate::project::{Ending, FileFacts};
use crate::run::{Diagnostics, Linting, Mode};
use crate::settings::FileSettings;

// The `full` settings a file's text is valid under: the display config and a digest of what else the text depends on.
// A file's text needs no run-wide state: the fix marker shows only on a diagnostic whose fix applies, and the run has
// such a fix only when one of its files does.
pub struct FullDisplay {
    config: DisplayDiagnosticConfig,
    base: u64,
}

impl FullDisplay {
    pub fn new(config: DisplayDiagnosticConfig, base: u64) -> Self {
        Self { config, base }
    }

    // The key a file's text is stored under, given the name the output shows the file by.
    pub fn key(&self, name: &str) -> u64 {
        let mut hasher = CacheKeyHasher::new();
        self.base.cache_key(&mut hasher);
        name.cache_key(&mut hasher);
        hasher.finish()
    }

    // The `full` text of one file's diagnostics, in the order the report sorts them.
    pub fn render(&self, diagnostics: &[Diagnostic]) -> Vec<u8> {
        let mut sorted = diagnostics.to_vec();
        sorted.sort_by(Diagnostic::ruff_start_ordering);
        let fixable = sorted.iter().any(|diagnostic| diagnostic.has_applicable_fix(self.config.fix_applicability()));
        let config = self.config.clone().with_show_fix_status(fixable);
        let notebooks = FxHashMap::default();
        let context = RenderContext::new(&notebooks);
        let mut buffer = Vec::new();
        render_diagnostics(&mut buffer, OutputFormat::Full, &config, &context, &sorted)
            .expect("writing to a vector cannot fail");
        buffer
    }
}

// The caches of a run, one per package root, or per directory for files outside a package, as Ruff keeps them. Their
// bytes are read up front and checked once by `views`, a package per thread, and read in place.
pub struct Caches(FxHashMap<PathBuf, Stored>);

// A package's cache file as read, with the root it is kept for.
struct Stored {
    path: PathBuf,
    bytes: Option<AlignedVec<16>>,
    // The package's `full` text, kept beside the cache file.
    rendered: PathBuf,
}

impl Caches {
    pub fn open<'p>(ruff_cache_dir: &Path, roots: impl Iterator<Item = &'p Path>) -> Self {
        let directory = build_directory(ruff_cache_dir);
        let roots: Vec<&Path> = roots.unique().collect();
        Self(roots.into_par_iter().map(|root| (root.to_path_buf(), Stored::read(&directory, root))).collect())
    }

    pub fn views(&self, run: u64) -> FxHashMap<&Path, Cache<'_>> {
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

// A package's stored `full` text, which a missing or unreadable file leaves empty.
fn read_rendered(path: &Path) -> FxHashMap<String, Rendered> {
    read_aligned(path)
        .ok()
        .and_then(|bytes| rkyv::from_bytes::<RenderedPackage, rkyv::rancor::Error>(&bytes).ok())
        .map(|package| package.files)
        .unwrap_or_default()
}

pub struct Cache<'a> {
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
        let now = SystemTime::UNIX_EPOCH.elapsed().map_or(0, |elapsed| elapsed.as_millis());
        Self {
            path: stored.path.clone(),
            package_root,
            archived,
            seen: Mutex::new(FxHashSet::default()),
            changes: Mutex::new(Vec::new()),
            now: u64::try_from(now).unwrap_or(u64::MAX),
            run,
            rendered: OnceLock::new(),
            rendered_path: stored.rendered.clone(),
            texts: Mutex::new(Vec::new()),
        }
    }

    // The import facts of a file that has not changed since they were cached, unless their ranges do not fit the file.
    pub fn facts(&self, path: &Path, key: FileKey) -> Option<FileFacts> {
        let stored = self.get(path, key)?.facts.as_ref()?;
        // rkyv reads the archive in place, which needs its bytes aligned.
        let mut bytes = AlignedVec::<16>::with_capacity(stored.len());
        bytes.extend_from_slice(stored);
        rkyv::from_bytes::<FileFacts, rkyv::rancor::Error>(&bytes)
            .inspect_err(|error| error!("Failed to read the facts of {}: {error}", path.display()))
            .ok()
            .filter(|facts| facts.fits(key.length))
    }

    pub fn set_facts(&self, path: &Path, key: FileKey, facts: &FileFacts) {
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(facts).expect("facts archive into memory");
        self.update(path, key, ChangeData::Facts(bytes.to_vec()));
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
        full: Option<&FullDisplay>,
        lint: impl FnOnce(Option<Withdrawn>) -> Linting,
    ) -> Diagnostics {
        let file = self.get(path, key);
        let index = IndexKey { run: self.run, base: fingerprint.base };
        // A result reused under the full fingerprint holds under this run's index too, so it is marked as such: a
        // later run with this index then reuses it without building the index, as it does results linted now.
        let relabel = |file: &ArchivedFileCache| {
            let current = file
                .index
                .as_ref()
                .is_some_and(|stored| stored.run.to_native() == index.run && stored.base.to_native() == index.base);
            if !current {
                self.update(path, key, ChangeData::Index(index));
            }
        };
        if let Some(file) = file
            && file.clean.as_ref().is_some_and(|clean| clean.to_native() == fingerprint.clean)
        {
            relabel(file);
            return Diagnostics::default();
        }
        if fingerprint.settled.is_none()
            && let Some(file) = file
            && let Some(found) =
                file.findings.as_ref().filter(|found| found.fingerprint.to_native() == fingerprint.clean)
            && let Some(mut diagnostics) = Findings::diagnostics(found, path)
        {
            relabel(file);
            if let Some(full) = full {
                self.stored_text(path, key, found.fingerprint.to_native(), full, &mut diagnostics);
            }
            return diagnostics;
        }
        let known = file
            .and_then(|file| file.settled.as_ref())
            .filter(|settled| fingerprint.settled == Some(settled.fingerprint.to_native()))
            .map(withdrawn);
        let fresh = known.is_none();
        let Linting { mut diagnostics, settled, noqa_rows } = lint(known);
        // Text is kept only for a file the cache already held reports for, so a first run that has nothing to reuse
        // renders the output as it did before, and the next run reuses the text.
        let held = file.is_some_and(|file| file.findings.is_some());
        let rendered = full
            .filter(|_| held && !diagnostics.inner.is_empty())
            .map(|full| text(full, &path.to_string_lossy(), &mut diagnostics, key.hash, fingerprint.clean));
        if diagnostics.inner.is_empty() && diagnostics.fixed.is_empty() {
            self.update(path, key, ChangeData::Clean(fingerprint.clean, index));
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
    pub fn reused(&self, path: &Path, key: FileKey, base: u64, full: Option<&FullDisplay>) -> Option<Diagnostics> {
        let file = self.get(path, key)?;
        let index = file.index.as_ref()?;
        if index.run.to_native() != self.run || index.base.to_native() != base {
            return None;
        }
        if file.clean.is_some() {
            return Some(Diagnostics::default());
        }
        let found = file.findings.as_ref()?;
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

    // Keeps the `full` text a run rendered for a file, which the package holds once the run writes its cache.
    fn store_text(&self, path: &Path, rendered: Rendered) {
        if let Some(relative) = self.relative(path) {
            self.texts.lock().expect("no worker panics while recording").push((relative.to_owned(), rendered));
        }
    }

    fn relative<'b>(&self, path: &'b Path) -> Option<&'b str> {
        path.strip_prefix(self.package_root).ok()?.to_str()
    }

    // The archived entry of a file whose key matches, which is then remembered as seen.
    fn get(&self, path: &Path, key: FileKey) -> Option<&'a ArchivedFileCache> {
        let relative = path.strip_prefix(self.package_root).ok()?.to_str()?;
        let file = self.archived?.files.get(relative)?;
        if file.key.to_native() != key.hash {
            return None;
        }
        self.seen.lock().expect("no worker panics while recording").insert(relative.to_owned());
        Some(file)
    }

    fn update(&self, path: &Path, key: FileKey, data: ChangeData) {
        if let Some(relative) = path.strip_prefix(self.package_root).ok().and_then(Path::to_str) {
            let change = Change { path: relative.to_owned(), key: key.hash, data };
            self.changes.lock().expect("no worker panics while recording").push(change);
        }
    }

    // Ruff's `persist`: files not seen for 30 days are dropped, and the cache goes to a temporary file renamed over the
    // old one, so a run killed midway or another simplint process never leaves a torn file. Nothing is written when
    // the run changed nothing, so the archive is only deserialized when there is something to write.
    pub fn persist(mut self, ending: Ending) -> Result<()> {
        let result = self.write();
        ending.end(self);
        result
    }

    fn write(&mut self) -> Result<()> {
        let changes = std::mem::take(self.changes.get_mut().expect("no worker panics while recording"));
        let texts = std::mem::take(self.texts.get_mut().expect("no worker panics while recording"));
        if changes.is_empty() && texts.is_empty() {
            return Ok(());
        }
        let now = self.now;
        let mut package = match self.archived {
            Some(archived) => PackageCache::read(archived)?,
            None => PackageCache { package_root: self.package_root.to_path_buf(), files: FxHashMap::default() },
        };
        for relative in self.seen.get_mut().expect("no worker panics while recording").drain() {
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
                ChangeData::Clean(fingerprint, index) => {
                    file.clean = Some(fingerprint);
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
    // The fingerprint the file last linted clean under.
    clean: Option<u64>,
    settled: Option<Settled>,
    // The reports the file last had without fixes, under their fingerprint.
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
            settled: None,
            findings: None,
            index: None,
        }
    }
}

// The digest of the import index a result was computed under, and the file's settings without its index slice.
#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct IndexKey {
    run: u64,
    base: u64,
}

// The reports a file had without fixes, under the fingerprint they were reported under.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Findings {
    fingerprint: u64,
    reports: Vec<Report>,
}

// The `full` text of a file's reports. It is valid for the file with `file` as its key, for the reports under
// `findings`, and for the display settings whose key is `key`.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Rendered {
    file: u64,
    findings: u64,
    key: u64,
    text: Vec<u8>,
}

// A package's `full` text, by the path of each file relative to the package root, kept beside its cache file. Runs that
// show other output never read it.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct RenderedPackage {
    files: FxHashMap<String, Rendered>,
}

// The `full` text of a file's diagnostics, which the diagnostics carry to the printer.
fn text(full: &FullDisplay, name: &str, diagnostics: &mut Diagnostics, file: u64, findings: u64) -> Rendered {
    let text = full.render(&diagnostics.inner);
    diagnostics.rendered.insert(name.to_owned(), text.clone());
    Rendered { file, findings, key: full.key(name), text }
}

impl Findings {
    // The diagnostics of an archived entry's reports, for the file that asks for them. The `full` text is kept beside
    // the entry, so a run that shows other output never reads it.
    fn diagnostics(archived: &ArchivedFindings, path: &Path) -> Option<Diagnostics> {
        let reports: Vec<Report> = rkyv::deserialize::<_, rkyv::rancor::Error>(&archived.reports).ok()?;
        // A file that cannot be read has no source to read back, and its report is built from none.
        let source = if reports.iter().all(|report| matches!(report.kind, ReportKind::IoError)) {
            String::new()
        } else {
            fs::read_to_string(path).ok()?
        };
        let file = SourceFileBuilder::new(path.to_string_lossy(), source.as_str()).finish();
        let inner = reports.iter().map(|report| report.diagnostic(&file)).collect::<Option<_>>()?;
        Some(Diagnostics { inner, ..Diagnostics::default() })
    }
}

// The rule of a code, looked up in a map since a warm run looks up every cached report.
fn rule_by_code(code: &str) -> Option<Rule> {
    static RULES: OnceLock<FxHashMap<&'static str, Rule>> = OnceLock::new();
    RULES.get_or_init(|| Rule::ALL.iter().map(|rule| (rule.code(), *rule)).collect()).get(code).copied()
}

// A diagnostic a rule reported, or one of the two a file gets that no rule reports, with the fields the output shows.
// `Report::of` takes only a diagnostic the checker builds, so `Report::diagnostic` gives back the same one.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Report {
    kind: ReportKind,
    message: String,
    start: u32,
    end: u32,
    help: Vec<String>,
    fix: Option<StoredFix>,
    // The other locations the report points at, in the order the checker gave them.
    related: Vec<StoredRelated>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum ReportKind {
    // The rule's code, and the line of its suppression comment, as `Suppressions::locate` sets it.
    Rule { code: String, noqa_offset: u32 },
    // A file that does not parse, whose syntax error `Linter::check` builds from its message and range alone.
    InvalidSyntax,
    // A file that cannot be read, which `Diagnostics::from_error` builds from its message alone.
    IoError,
}

// A location a report points at besides its primary one, as the file it is in, its byte range and its message.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct StoredRelated {
    path: String,
    start: u32,
    end: u32,
    message: String,
}

impl StoredRelated {
    // The location a secondary annotation names, or `None` when it is not one the checker set: a file its source
    // cannot be read back from, or one without a message.
    fn of(annotation: &Annotation) -> Option<Self> {
        let span = annotation.get_span();
        let UnifiedFile::Ruff(file) = span.file() else { return None };
        let range = span.range()?;
        Some(Self {
            path: file.name().to_owned(),
            start: range.start().to_u32(),
            end: range.end().to_u32(),
            message: annotation.get_message()?.to_owned(),
        })
    }
}

impl Report {
    // All of the reports, or `None` when a diagnostic is one `of` cannot store. Only a rule's diagnostic has a
    // suppression line, so a file's syntax error or unreadable file takes none.
    fn all(diagnostics: &[Diagnostic], noqa_rows: &[TextSize]) -> Option<Vec<Self>> {
        let mut rows = noqa_rows.iter().copied();
        diagnostics.iter().map(|diagnostic| Self::of(diagnostic, rows.next())).collect()
    }

    // The report of a diagnostic. Only what the checker sets is stored: a rule's diagnostic with more than its one
    // primary annotation, a parent, a severity other than error, or a sub-diagnostic other than a help note is not.
    fn of(diagnostic: &Diagnostic, noqa_row: Option<TextSize>) -> Option<Self> {
        let primary = diagnostic.annotations().first()?;
        let range = primary.get_span().range()?;
        let unruled = |kind| {
            Some(Self {
                kind,
                message: diagnostic.headline_message().to_owned(),
                start: range.start().to_u32(),
                end: range.end().to_u32(),
                help: Vec::new(),
                fix: None,
                related: Vec::new(),
            })
        };
        if diagnostic.is_invalid_syntax() {
            return unruled(ReportKind::InvalidSyntax);
        }
        if diagnostic.id() == DiagnosticId::Lint(RuffRule::IOError.name()) {
            return unruled(ReportKind::IoError);
        }
        let rule = rule_by_code(diagnostic.secondary_code()?.as_str())?;
        let url = rule.url();
        let related: Vec<StoredRelated> =
            diagnostic.secondary_annotations().map(StoredRelated::of).collect::<Option<_>>()?;
        let help: Vec<String> = diagnostic
            .sub_diagnostics()
            .iter()
            .map(|sub| {
                let help = sub.severity() == SubDiagnosticSeverity::Help && sub.annotations().is_empty();
                help.then(|| sub.headline_message().to_owned())
            })
            .collect::<Option<_>>()?;
        let plain = diagnostic.annotations().iter().filter(|annotation| annotation.is_primary()).count() == 1
            && primary.is_primary()
            && primary.get_message().is_none()
            && diagnostic.primary_tags().is_none_or(<[_]>::is_empty)
            && diagnostic.id() == DiagnosticId::Lint(LintName::of(rule.name()))
            && diagnostic.severity() == Severity::Error
            && diagnostic.documentation_url() == Some(url.as_str())
            && diagnostic.parent().is_none();
        let fix = diagnostic.fix().map(StoredFix::of);
        let noqa_offset = noqa_row?.to_u32();
        plain.then(|| Self {
            kind: ReportKind::Rule { code: rule.code().to_owned(), noqa_offset },
            message: diagnostic.headline_message().to_owned(),
            start: range.start().to_u32(),
            end: range.end().to_u32(),
            help,
            fix,
            related,
        })
    }

    // The diagnostic `of` took the report from, given the file it reports on, or `None` when a range does not fit the
    // file as it now reads.
    fn diagnostic(&self, file: &SourceFile) -> Option<Diagnostic> {
        let length = file.source_text().len();
        let range = stored_range(self.start, self.end, length)?;
        let (rule, noqa_offset) = match &self.kind {
            ReportKind::InvalidSyntax => {
                return Some(Diagnostic::invalid_syntax(Span::from(file.clone()), self.message.as_str(), range));
            }
            ReportKind::IoError => {
                return Some(IOError { message: self.message.clone() }.into_diagnostic(range, file));
            }
            ReportKind::Rule { code, noqa_offset } => (rule_by_code(code)?, *noqa_offset),
        };
        stored_range(noqa_offset, noqa_offset, length)?;
        let mut diagnostic =
            Diagnostic::new(DiagnosticId::Lint(LintName::of(rule.name())), Severity::Error, self.message.as_str());
        diagnostic.annotate(Annotation::primary(Span::from(file.clone()).with_range(range)));
        for related in &self.related {
            let file = source_file_at(Path::new(&related.path))?;
            let range = stored_range(related.start, related.end, file.source_text().len())?;
            let span = Span::from(file).with_range(range);
            diagnostic.annotate(Annotation::secondary(span).message(related.message.as_str()));
        }
        diagnostic.set_secondary_code(SecondaryCode::new(rule.code().to_owned()));
        diagnostic.set_documentation_url(Some(rule.url()));
        for help in &self.help {
            diagnostic.help(help.as_str());
        }
        if let Some(fix) = &self.fix {
            diagnostic.set_fix(fix.fix(length)?);
        }
        diagnostic.set_noqa_offset(TextSize::new(noqa_offset));
        Some(diagnostic)
    }
}

// A fix as rkyv data. Ruff's `Fix` keeps its fields private, so `fix` rebuilds it from its edits and levels, in the
// order they were stored, which is the order `Fix` sorts them into.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct StoredFix {
    applicability: StoredApplicability,
    isolation: StoredIsolation,
    edits: Vec<StoredEdit>,
}

impl StoredFix {
    fn of(fix: &Fix) -> Self {
        Self {
            applicability: match fix.applicability() {
                Applicability::DisplayOnly => StoredApplicability::DisplayOnly,
                Applicability::Unsafe => StoredApplicability::Unsafe,
                Applicability::Safe => StoredApplicability::Safe,
            },
            isolation: match fix.isolation() {
                IsolationLevel::Group(group) => StoredIsolation::Group(group),
                IsolationLevel::NonOverlapping => StoredIsolation::NonOverlapping,
            },
            edits: fix
                .edits()
                .iter()
                .map(|edit| StoredEdit {
                    start: edit.start().to_u32(),
                    end: edit.end().to_u32(),
                    content: edit.content().map(str::to_owned),
                })
                .collect(),
        }
    }

    // `None` for a fix without edits, which no rule builds and which a cache file therefore cannot hold, or with an edit
    // outside a source of `length` bytes.
    fn fix(&self, length: usize) -> Option<Fix> {
        let applicability = match self.applicability {
            StoredApplicability::DisplayOnly => Applicability::DisplayOnly,
            StoredApplicability::Unsafe => Applicability::Unsafe,
            StoredApplicability::Safe => Applicability::Safe,
        };
        let isolation = match self.isolation {
            StoredIsolation::Group(group) => IsolationLevel::Group(group),
            StoredIsolation::NonOverlapping => IsolationLevel::NonOverlapping,
        };
        let edits: Vec<Edit> = self.edits.iter().map(|edit| edit.edit(length)).collect::<Option<_>>()?;
        let (first, rest) = edits.split_first()?;
        Some(Fix::applicable_edits(first.clone(), rest.iter().cloned(), applicability).isolate(isolation))
    }
}

#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum StoredApplicability {
    DisplayOnly,
    Unsafe,
    Safe,
}

#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum StoredIsolation {
    Group(u32),
    NonOverlapping,
}

// An edit's range, and its replacement content, `None` for a deletion.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct StoredEdit {
    start: u32,
    end: u32,
    content: Option<String>,
}

impl StoredEdit {
    fn edit(&self, length: usize) -> Option<Edit> {
        let range = stored_range(self.start, self.end, length)?;
        Some(match &self.content {
            Some(content) => Edit::range_replacement(content.clone(), range),
            None => Edit::range_deletion(range),
        })
    }
}

// A range read back from a cache file, or `None` when it cannot lie in a source of `length` bytes, as in a cache file
// that was damaged or edited by hand, or a file changed without changing its key.
fn stored_range(start: u32, end: u32, length: usize) -> Option<TextRange> {
    (start <= end && usize::try_from(end).is_ok_and(|end| end <= length))
        .then(|| TextRange::new(TextSize::new(start), TextSize::new(end)))
}

// The fix loop's last run over an unchanged file: the fingerprint it ran under and the fixes it withdrew, rules by code
// and single fixes by rule code, message and text. Codes, unlike a rule's position in the rule set, stay put while
// development builds of a version add rules.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct Settled {
    fingerprint: u64,
    rules: Vec<String>,
    fixes: Vec<WithdrawnFix>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct WithdrawnFix {
    rule: String,
    message: String,
    text: String,
    ordinal: usize,
}

impl Settled {
    fn new(fingerprint: u64, withdrawn: &Withdrawn) -> Self {
        Self {
            fingerprint,
            rules: withdrawn.rules.iter().map(|rule| rule.code().to_owned()).collect(),
            fixes: withdrawn
                .fixes
                .iter()
                .map(|(rule, message, text, ordinal)| WithdrawnFix {
                    rule: rule.code().to_owned(),
                    message: message.clone(),
                    text: text.clone(),
                    ordinal: *ordinal,
                })
                .collect(),
        }
    }
}

fn withdrawn(settled: &ArchivedSettled) -> Withdrawn {
    Withdrawn {
        rules: RuleSet::from_rules(settled.rules.iter().filter_map(|code| Rule::from_code(code.as_str()))),
        fixes: settled
            .fixes
            .iter()
            .filter_map(|fix| {
                Some((
                    Rule::from_code(fix.rule.as_str())?,
                    fix.message.as_str().to_owned(),
                    fix.text.as_str().to_owned(),
                    fix.ordinal.to_native() as usize,
                ))
            })
            .collect(),
    }
}

struct Change {
    path: String,
    key: u64,
    data: ChangeData,
}

enum ChangeData {
    Facts(Vec<u8>),
    Clean(u64, IndexKey),
    Findings(u64, Vec<Report>, IndexKey),
    Settled(u64, Withdrawn),
    Index(IndexKey),
}

// A file's fingerprint: its settings, where it sits in its package, and its slice of the project index. Most files
// share their project's settings and Ruff's, so those, Ruff's linter settings above all, are hashed once per pair.
#[derive(Default)]
pub struct Fingerprints(Mutex<FxHashMap<(usize, usize), u64>>);

impl Fingerprints {
    // A file's settings, where it sits in its package and its sibling `__init__.py`, which the index does not read.
    // The run's digest covers the index, so `base` with it says whether a stored result is still valid.
    pub fn base(
        &self,
        settings: &FileSettings,
        ruff: &LinterSettings,
        module_path: Option<&Vec<String>>,
        sibling_init: bool,
    ) -> u64 {
        let shared = *self
            .0
            .lock()
            .expect("no worker panics while fingerprinting")
            .entry((std::ptr::from_ref(settings.project).addr(), std::ptr::from_ref(ruff).addr()))
            .or_insert_with(|| {
                let mut hasher = CacheKeyHasher::new();
                settings.project.cache_key(&mut hasher);
                ruff.cache_key(&mut hasher);
                hasher.finish()
            });
        let mut hasher = CacheKeyHasher::new();
        shared.cache_key(&mut hasher);
        let rules = settings.rules;
        rules.cache_key(&mut hasher);
        settings.from_ruff.cache_key(&mut hasher);
        // An `__init__.py` added or removed next to the file moves its package root, which keeps the cache file. In a
        // namespace package that root stays, so the rules that read the file's `__init__.py` itself key it too.
        module_path.cache_key(&mut hasher);
        if rules.overlaps(RuleSet::sibling_init()) {
            sibling_init.cache_key(&mut hasher);
        }
        hasher.finish()
    }

    pub fn of(&self, linter: &Linter, mode: &Mode) -> Fingerprint {
        let rules = linter.settings.rules;
        let base = self.base(&linter.settings, linter.ruff.settings, linter.module_path.as_ref(), linter.sibling_init);
        // Only a rule that reads other files depends on the index of the run, so the others keep their results when
        // another file changes.
        let clean = if rules.overlaps(RuleSet::cross_file()) {
            let mut hasher = CacheKeyHasher::new();
            base.cache_key(&mut hasher);
            linter.project.cache_key(&mut hasher);
            hasher.finish()
        } else {
            base
        };
        // Which fixes apply decides what the fix loop withdraws, and whether it changes the file at all.
        let settled = (!mode.fix.is_generate()).then(|| {
            let mut hasher = CacheKeyHasher::new();
            clean.cache_key(&mut hasher);
            mode.fix.hash(&mut hasher);
            mode.unsafe_fixes.cache_key(&mut hasher);
            mode.noqa.hash(&mut hasher);
            hasher.finish()
        });
        Fingerprint { clean, settled, base }
    }
}

// What a clean result is valid under, and with fixes on, what a settled fix loop is valid under.
#[derive(Clone, Copy)]
pub struct Fingerprint {
    clean: u64,
    settled: Option<u64>,
    base: u64,
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
    use ruff_text_size::{TextRange, TextSize};
    use test_case::test_case;

    use ruff_source_file::SourceFileBuilder;

    use super::{Report, ReportKind, StoredApplicability, StoredEdit, StoredFix, StoredIsolation};

    const SOURCE_LENGTH: usize = 30;

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::new(start), TextSize::new(end))
    }

    // The fix a cache file holds after an rkyv round trip, which is what a later run reads back.
    fn round_trip(fix: &Fix) -> Option<Fix> {
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&StoredFix::of(fix)).expect("serialize");
        rkyv::from_bytes::<StoredFix, rkyv::rancor::Error>(&bytes).expect("deserialize").fix(SOURCE_LENGTH)
    }

    #[test_case(Fix::safe_edit(Edit::range_replacement("y".to_owned(), range(4, 9))) ; "safe_replacement")]
    #[test_case(
        Fix::unsafe_edits(
            Edit::range_deletion(range(0, 3)),
            [Edit::insertion("import os\n".to_owned(), TextSize::new(12)), Edit::range_replacement("z".to_owned(), range(20, 25))],
        )
        .isolate(IsolationLevel::Group(7)) ; "unsafe_edits_in_a_group"
    )]
    #[test_case(Fix::display_only_edit(Edit::deletion(TextSize::new(2), TextSize::new(2))) ; "display_only_deletion")]
    fn fix_survives_a_round_trip(fix: Fix) {
        assert_eq!(round_trip(&fix), Some(fix));
    }

    #[test]
    fn fix_without_edits_is_not_restored() {
        let stored = StoredFix {
            applicability: StoredApplicability::Safe,
            isolation: StoredIsolation::NonOverlapping,
            edits: Vec::new(),
        };
        assert!(stored.fix(SOURCE_LENGTH).is_none());
    }

    #[test_case(0, 1, 0, true ; "fits")]
    #[test_case(1, 0, 0, false ; "end_before_start")]
    #[test_case(0, 12, 0, false ; "end_past_the_source")]
    #[test_case(0, 1, 12, false ; "suppression_past_the_source")]
    fn report_is_restored_only_inside_the_source(start: u32, end: u32, noqa_offset: u32, restored: bool) {
        let file = SourceFileBuilder::new("module.py", "x = 1\ny = 2").finish();
        let report = Report {
            kind: ReportKind::Rule { code: "SMP101".to_owned(), noqa_offset },
            message: "Single-letter name".to_owned(),
            start,
            end,
            help: Vec::new(),
            fix: None,
            related: Vec::new(),
        };
        assert_eq!(report.diagnostic(&file).is_some(), restored);
    }

    #[test_case(5, 3 ; "end_before_start")]
    #[test_case(3, 31 ; "end_past_the_source")]
    fn fix_with_an_edit_outside_the_source_is_not_restored(start: u32, end: u32) {
        let stored = StoredFix {
            applicability: StoredApplicability::Safe,
            isolation: StoredIsolation::NonOverlapping,
            edits: vec![StoredEdit { start, end, content: None }],
        };
        assert!(stored.fix(SOURCE_LENGTH).is_none());
    }

    #[test]
    fn applicability_and_isolation_are_kept() {
        let fix = Fix::applicable_edit(Edit::deletion(TextSize::new(0), TextSize::new(1)), Applicability::Unsafe)
            .isolate(IsolationLevel::NonOverlapping);
        assert_eq!(
            round_trip(&fix).map(|fix| (fix.applicability(), fix.isolation())),
            Some((Applicability::Unsafe, IsolationLevel::NonOverlapping))
        );
    }
}
