// The lint pipeline and the run over a set of files, which the command line reads and the project cache stores.
mod cache;
mod check;
mod conflicts;
mod digest;
mod discover;
mod files;
mod overlay;
mod rounds;

use std::env;
use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::ops::{Add, AddAssign};
use std::path::{Path, PathBuf};

use anyhow::{Result, ensure};
use colored::Colorize;
use log::warn;
use ruff::args::ConfigArguments;
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::fs::relativize_path;
use ruff_linter::linter::FixTable;
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::LinterSettings;
use ruff_linter::settings::flags;
use ruff_linter::settings::types::UnsafeFixes;
use ruff_linter::source_kind::SourceKindDiff;
use ruff_linter::{IOError, Violation, warn_user};
use ruff_source_file::SourceFileBuilder;
use ruff_text_size::{TextRange, TextSize};
use ruff_workspace::resolver::PyprojectConfig;
use rustc_hash::FxHashMap;

use crate::fix::{Fixer, FixerResult, Withdrawn, add_fixes};
use crate::lint::Linter;
use crate::settings::SettingsResolver;

pub(crate) use cache::{DisplayKey, FullDisplay};
pub(crate) use check::{CacheUse, add_suppressions, check_files, check_stdin};
use discover::within;
pub(crate) use discover::{Discovered, discover, linted_roots, redact_extended, warn_missing};
pub(crate) use overlay::Overlay;

pub(crate) struct Resolved {
    pub(crate) config_arguments: ConfigArguments,
    pub(crate) settings_resolver: SettingsResolver,
    pub(crate) pyproject_config: PyprojectConfig,
    pub(crate) cache_dir: Option<PathBuf>,
}

impl Resolved {
    // Ruff's `output-prefer-rule-codes`: in preview, messages name rules unless it is set.
    fn names_rules(&self) -> bool {
        self.pyproject_config.settings.linter.preview.is_enabled()
            && !self.pyproject_config.settings.output_prefer_rule_codes
    }
}

// Ruff's cache directory, where simplint keeps its cache. A path from the command line or `RUFF_CACHE_DIR` is the
// user's choice and stands wherever it points. Otherwise the repository being linted picks it, through `cache-dir` or a
// link committed in place of the default, so it must stay inside the project root: the directory of the configuration
// file, or the working directory without one. A `cache-dir` that leaves the root falls back to Ruff's default there,
// and a default that leaves it too turns the cache off.
pub(crate) fn choose_cache_dir(named: bool, config: &PyprojectConfig) -> Option<PathBuf> {
    let configured = &config.settings.cache_dir;
    if named {
        return Some(configured.clone());
    }
    let root = match config.path.as_deref().and_then(Path::parent) {
        Some(parent) => parent.to_path_buf(),
        None => env::current_dir().ok()?,
    };
    let chosen = if within(configured, &root) { configured.clone() } else { ruff_cache::cache_dir(&root) };
    // The directory of this build is the deepest the cache creates, so checking it covers a link at any level above.
    let build = crate::project::cache::documentation_file(&chosen, &root);
    if build.parent().is_some_and(|build| within(build, &root)) {
        return Some(chosen);
    }
    warn_user!("Not caching: the cache directory `{}` leads outside the project", chosen.display());
    None
}

#[derive(Default)]
pub(crate) struct Diagnostics {
    pub(crate) inner: Vec<Diagnostic>,
    pub(crate) fixed: FixMap,
    pub(crate) diffs: Vec<(String, String)>,
    // The `full` text of some files' diagnostics, by the name the output shows each file by, when the cache held or
    // rendered it.
    pub(crate) rendered: FxHashMap<String, Vec<u8>>,
}

impl Diagnostics {
    // As `ruff check` reports a file it cannot read or write: Ruff's own `io-error` diagnostic while that rule is on,
    // otherwise a warning.
    fn from_error(path: &Path, message: String, linter: &Linter) -> Self {
        if !reports_io_errors(linter.ruff.settings) {
            warn!("{}{}{} {message}", "Failed to lint ".bold(), relativize_path(path).bold(), ":".bold());
            return Self::default();
        }
        Self { inner: vec![io_error(path, message)], ..Self::default() }
    }

    // A later pass over a file replaces its reports, while the fixes of every pass count. Each pass's diff is from the
    // source as read, so the last pass that has one holds all of the file's fixes.
    pub(crate) fn then(mut self, later: Self) -> Self {
        self.inner = later.inner;
        self.fixed += later.fixed;
        if !later.diffs.is_empty() {
            self.diffs = later.diffs;
        }
        self
    }
}

// Whether Ruff's `io-error` rule is on, which reports a path that cannot be read, written or found as a diagnostic
// rather than as a warning.
fn reports_io_errors(settings: &LinterSettings) -> bool {
    settings.rules.enabled(RuffRule::IOError)
}

// Ruff's `io-error` diagnostic for a path that cannot be read, written or found.
fn io_error(path: &Path, message: String) -> Diagnostic {
    let file = SourceFileBuilder::new(path.to_string_lossy(), "").finish();
    IOError { message }.into_diagnostic(TextRange::default(), &file)
}

impl Add for Diagnostics {
    type Output = Self;

    fn add(mut self, other: Self) -> Self {
        self += other;
        self
    }
}

impl AddAssign for Diagnostics {
    fn add_assign(&mut self, other: Self) {
        self.inner.extend(other.inner);
        self.fixed += other.fixed;
        self.diffs.extend(other.diffs);
        self.rendered.extend(other.rendered);
    }
}

#[derive(Default)]
pub(crate) struct FixMap(FxHashMap<String, FixTable>);

impl FixMap {
    pub(crate) fn of(path: &Path, fixed: FixTable) -> Self {
        Self::from_iter([(relativize_path(path), fixed)])
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&String, &FixTable)> {
        self.0.iter()
    }

    pub(crate) fn values(&self) -> impl Iterator<Item = &FixTable> {
        self.0.values()
    }
}

impl FromIterator<(String, FixTable)> for FixMap {
    fn from_iter<T: IntoIterator<Item = (String, FixTable)>>(iter: T) -> Self {
        Self(iter.into_iter().filter(|(_, fixes)| !fixes.is_empty()).collect())
    }
}

impl AddAssign for FixMap {
    fn add_assign(&mut self, other: Self) {
        #[expect(clippy::iter_over_hash_type, reason = "fix counts merge independently for each file")]
        for (filename, fixed) in other.0 {
            add_fixes(self.0.entry(filename).or_default(), &fixed);
        }
    }
}

pub(crate) struct Mode {
    pub(crate) fix: flags::FixMode,
    pub(crate) unsafe_fixes: UnsafeFixes,
    pub(crate) noqa: flags::Noqa,
    // The paths named on the command line, canonical, which a fix may write files under.
    pub(crate) roots: Vec<PathBuf>,
}

// A file's lint: its diagnostics and fixes, the fixes a settled fix loop withdrew, and, without fixes, the line each
// diagnostic's suppression comment belongs on, which the cache stores with the reports.
//
// A file whose fixes cannot be written is not cacheable: the cache keys a file by its metadata, not by whether its
// directory and the run's roots let it be written, so its reports would outlive what they depend on. A file that
// cannot be read fails the same way for the same key, so its report is stored, unless the error is only logged and so
// leaves no report. `unparsed` marks a file that does not parse, which has no diagnostics but its warning.
struct Linting {
    pub(crate) diagnostics: Diagnostics,
    pub(crate) settled: Option<Withdrawn>,
    pub(crate) noqa_rows: Vec<TextSize>,
    cacheable: bool,
    unparsed: bool,
}

// Ruff's `lint_path`: lint a file, and with `--fix` or `--diff` fix it and write the result back or keep the diff for
// the caller to print. A `--diff` run keeps the fixed source in `overlay` instead of writing it, so the next fix round
// lints what the fixes made. Files are linted in parallel, so the diff waits for the caller to print it in file order.
//
// A file that cannot be read is reported in place of its diagnostics, as Ruff reports `io-error`; one whose fixes
// cannot be written keeps its diagnostics, with the error reported beside them. Both `settled` sets name the fixes the
// fix loop withdrew from a file it left as it was: the one passed from an earlier run, which spares running the loop,
// the one returned from this run.
fn lint_path(path: &Path, linter: &Linter, mode: &Mode, settled: Option<Withdrawn>, overlay: &Overlay) -> Linting {
    fix_path(path, linter, mode, settled, overlay).unwrap_or_else(|error| {
        let diagnostics = Diagnostics::from_error(path, format!("{error:#}"), linter);
        // A cached clean result would leave out the warning of a read error that was only logged.
        Linting {
            cacheable: !diagnostics.inner.is_empty(),
            diagnostics,
            settled: None,
            noqa_rows: Vec::new(),
            unparsed: false,
        }
    })
}

fn fix_path(
    path: &Path,
    linter: &Linter,
    mode: &Mode,
    settled: Option<Withdrawn>,
    overlay: &Overlay,
) -> Result<Linting> {
    let source = overlay.source(path)?;
    let Linted { mut diagnostics, transformed, fixed, settled, noqa_rows } =
        lint_source(path, &source, linter, mode, settled);
    let unparsed = skip_unparsed(path, &mut diagnostics);
    let mut diffs = Vec::new();
    if let Some(transformed) = transformed.filter(|_| !fixed.is_empty()) {
        if mode.fix.is_apply() {
            if let Err(error) = write_source(path, &mode.roots, &source, &transformed) {
                return Ok(unwritten(path, &source, linter, mode, &error));
            }
        } else {
            let diff = overlay.record(path, &source, &transformed);
            diffs.push((path.to_string_lossy().into_owned(), diff));
        }
    }
    let fixed = FixMap::of(path, fixed);
    Ok(Linting {
        diagnostics: Diagnostics { inner: diagnostics, fixed, diffs, ..Diagnostics::default() },
        settled,
        noqa_rows,
        cacheable: true,
        unparsed,
    })
}

// Replaces a file read as `original` with `fixed`, in place as Ruff writes it, so the file keeps its owner, hard
// links and extended attributes. A file edited since it was read keeps the edit. A symlink keeps pointing at the file
// it names, which gets the new source, as long as that file lies under a root of the run.
fn write_source(path: &Path, roots: &[PathBuf], original: &str, fixed: &str) -> Result<()> {
    let target = std::fs::canonicalize(path)?;
    ensure!(roots.iter().any(|root| target.starts_with(root)), "{} lies outside the linted paths", path.display());
    let mut file = OpenOptions::new().read(true).write(true).open(&target)?;
    let mut current = String::new();
    file.read_to_string(&mut current)?;
    ensure!(current == original, "{} changed while simplint fixed it", path.display());
    file.seek(SeekFrom::Start(0))?;
    file.set_len(0)?;
    file.write_all(fixed.as_bytes())?;
    file.sync_all()?;
    Ok(())
}

// A fix that cannot be written leaves the file as it is, so its reports are those of the source as read, with the
// write's error reported beside them and no fix counted.
fn unwritten(path: &Path, source: &str, linter: &Linter, mode: &Mode, error: &anyhow::Error) -> Linting {
    let lint = linter.lint(path, source, mode.noqa);
    let mut diagnostics = Diagnostics::from_error(path, format!("writing {}: {error:#}", path.display()), linter);
    diagnostics.inner.extend(lint.diagnostics);
    Linting { diagnostics, settled: None, noqa_rows: Vec::new(), cacheable: false, unparsed: false }
}

pub(crate) struct StdinLint {
    pub(crate) diagnostics: Diagnostics,
    pub(crate) output: Option<String>,
}

// Ruff's `lint_stdin`: the source comes from standard input, and with `--fix` the source, fixed or not, goes to
// standard output.
fn lint_stdin(path: &Path, source: &str, linter: &Linter, mode: &Mode) -> StdinLint {
    let Linted { mut diagnostics, transformed, fixed, .. } = lint_source(path, source, linter, mode, None);
    skip_unparsed(path, &mut diagnostics);
    let output = if mode.fix.is_apply() {
        Some(transformed.unwrap_or_else(|| source.to_owned()))
    } else {
        transformed
            .filter(|_| !fixed.is_empty())
            .map(|transformed| SourceKindDiff::from_text(source, &transformed, Some(path)).to_string())
    };
    StdinLint {
        diagnostics: Diagnostics {
            inner: diagnostics,
            fixed: FixMap::of(path, fixed),
            diffs: Vec::new(),
            rendered: FxHashMap::default(),
        },
        output,
    }
}

// A fix that fails leaves the file as it was, so its diagnostics come from the original source, with nothing fixed. A
// source the fix loop already settled, whose withdrawn fixes `settled` names, gets the loop's reports from one lint.
fn lint_source(path: &Path, source: &str, linter: &Linter, mode: &Mode, settled: Option<Withdrawn>) -> Linted {
    let unfixed = || {
        let lint = linter.lint(path, source, mode.noqa);
        Linted {
            diagnostics: lint.diagnostics,
            transformed: None,
            fixed: FixTable::default(),
            settled: None,
            noqa_rows: lint.noqa_rows,
        }
    };
    if mode.fix.is_generate() {
        return unfixed();
    }
    if let Some(withdrawn) = settled {
        let mut linted = unfixed();
        Fixer::withdraw(source, &mut linted.diagnostics, &withdrawn);
        return Linted { settled: Some(withdrawn), ..linted };
    }
    let result = Fixer::new(linter, path, mode.unsafe_fixes, mode.noqa).fix(source);
    let Ok((FixerResult { diagnostics, transformed, fixed }, withdrawn)) = result else { return unfixed() };
    let settled = (fixed.is_empty() && transformed == source).then_some(withdrawn);
    Linted { diagnostics, transformed: Some(transformed), fixed, settled, noqa_rows: Vec::new() }
}

// A file that does not parse has no reports, since Ruff reports its syntax errors and the rules need a valid tree. Its
// warning comes from here and from a cached result alike, so each run shows it.
fn skip_unparsed(path: &Path, diagnostics: &mut Vec<Diagnostic>) -> bool {
    let unparsed = diagnostics.iter().any(Diagnostic::is_invalid_syntax);
    if unparsed {
        diagnostics.clear();
        warn_unparsed(path);
    }
    unparsed
}

fn warn_unparsed(path: &Path) {
    warn_user!("Skipped {}: it does not parse, so Ruff reports its syntax errors.", relativize_path(path));
}

struct Linted {
    diagnostics: Vec<Diagnostic>,
    transformed: Option<String>,
    fixed: FixTable,
    // `settled` names the fixes the fix loop withdrew when it left the source as it was.
    settled: Option<Withdrawn>,
    // Only a lint without fixes keeps these, aligned with `diagnostics`.
    noqa_rows: Vec<TextSize>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::write_source;

    #[test]
    fn write_source_keeps_a_file_changed_since_it_was_read() {
        let directory = tempfile::TempDir::new().expect("temporary directory");
        let path = directory.path().join("module.py");
        std::fs::write(&path, "edited = 1\n").expect("write file");
        let roots = [std::fs::canonicalize(directory.path()).expect("canonical directory")];
        let error = write_source(&path, &roots, "read = 1\n", "fixed = 1\n").expect_err("the file changed");
        assert_eq!(
            (error.to_string(), std::fs::read_to_string(&path).expect("read file")),
            (format!("{} changed while simplint fixed it", path.display()), "edited = 1\n".to_owned())
        );
    }

    #[test]
    fn write_source_refuses_a_file_outside_the_roots() {
        let root = tempfile::TempDir::new().expect("temporary directory");
        let outside = tempfile::TempDir::new().expect("temporary directory");
        let path = outside.path().join("module.py");
        std::fs::write(&path, "read = 1\n").expect("write file");
        let roots: Vec<PathBuf> = vec![std::fs::canonicalize(root.path()).expect("canonical directory")];
        let error = write_source(&path, &roots, "read = 1\n", "fixed = 1\n").expect_err("outside the roots");
        assert_eq!(
            (error.to_string(), std::fs::read_to_string(&path).expect("read file")),
            (format!("{} lies outside the linted paths", path.display()), "read = 1\n".to_owned())
        );
    }
}
