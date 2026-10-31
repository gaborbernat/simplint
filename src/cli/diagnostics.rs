use std::io::Write;
use std::ops::{Add, AddAssign};
use std::path::Path;

use anyhow::{Context, Result};
use colored::Colorize;
use log::warn;
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::fs::relativize_path;
use ruff_linter::linter::FixTable;
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::flags;
use ruff_linter::settings::types::UnsafeFixes;
use ruff_linter::source_kind::SourceKindDiff;
use ruff_linter::{IOError, Violation};
use ruff_source_file::SourceFileBuilder;
use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use crate::fix::{Fixer, FixerResult};
use crate::lint::Linter;
use crate::lint::registry::RuleSet;

// What a run reports: the remaining diagnostics and the fixes applied per file, as Ruff's `Diagnostics` holds them.
#[derive(Default)]
pub struct Diagnostics {
    pub inner: Vec<Diagnostic>,
    pub fixed: FixMap,
}

impl Diagnostics {
    // As `ruff check` reports a file it cannot read or write: Ruff's own `io-error` diagnostic while that rule is on,
    // otherwise a warning.
    pub fn from_error(path: &Path, message: String, linter: &Linter) -> Self {
        if !linter.ruff.settings.rules.enabled(RuffRule::IOError) {
            warn!("{}{}{} {message}", "Failed to lint ".bold(), relativize_path(path).bold(), ":".bold());
            return Self::default();
        }
        let file = SourceFileBuilder::new(path.to_string_lossy(), "").finish();
        let diagnostic = IOError { message }.into_diagnostic(TextRange::default(), &file);
        Self { inner: vec![diagnostic], fixed: FixMap::default() }
    }
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
    }
}

#[derive(Default)]
pub struct FixMap(FxHashMap<String, FixTable>);

impl FixMap {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &FixTable)> {
        self.0.iter()
    }

    pub fn values(&self) -> impl Iterator<Item = &FixTable> {
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
            let fixed_in_file = self.0.entry(filename).or_default();
            for (id, code, count) in fixed.iter() {
                *fixed_in_file.entry(id).or_default(code) += count;
            }
        }
    }
}

pub struct Mode {
    pub fix: flags::FixMode,
    pub unsafe_fixes: UnsafeFixes,
    pub noqa: flags::Noqa,
}

// Ruff's `lint_path`: lint a file, and with `--fix` or `--diff` fix it and write the result back or print the diff. A
// file that cannot be read, or whose fix cannot be written or printed, is reported in place of its diagnostics, as Ruff
// reports `io-error`. Both `settled` sets name the rules whose fixes the fix loop withdrew from a file it left as it
// was: the one passed from an earlier run, which spares running the loop, the one returned from this run.
pub fn lint_path(
    path: &Path,
    linter: &Linter,
    mode: &Mode,
    settled: Option<RuleSet>,
) -> (Diagnostics, Option<RuleSet>) {
    fix_path(path, linter, mode, settled)
        .unwrap_or_else(|error| (Diagnostics::from_error(path, format!("{error:#}"), linter), None))
}

fn fix_path(
    path: &Path,
    linter: &Linter,
    mode: &Mode,
    settled: Option<RuleSet>,
) -> Result<(Diagnostics, Option<RuleSet>)> {
    let source = std::fs::read_to_string(path)?;
    let Linted { diagnostics, transformed, fixed, settled } = lint_source(path, &source, linter, mode, settled);
    if let Some(transformed) = transformed.filter(|_| !fixed.is_empty()) {
        if mode.fix.is_apply() {
            std::fs::write(path, &transformed).with_context(|| format!("writing {}", path.display()))?;
        } else {
            write!(std::io::stdout().lock(), "{}", SourceKindDiff::from_text(&source, &transformed, Some(path)))?;
        }
    }
    let diagnostics = Diagnostics { inner: diagnostics, fixed: FixMap::from_iter([(relativize_path(path), fixed)]) };
    Ok((diagnostics, settled))
}

// Ruff's `lint_stdin`: the source comes from standard input, and with `--fix` the source, fixed or not, goes to
// standard output.
pub fn lint_stdin(path: &Path, source: &str, linter: &Linter, mode: &Mode) -> Result<Diagnostics> {
    let Linted { diagnostics, transformed, fixed, .. } = lint_source(path, source, linter, mode, None);
    let mut stdout = std::io::stdout().lock();
    if mode.fix.is_apply() {
        stdout.write_all(transformed.as_deref().unwrap_or(source).as_bytes())?;
    } else if let Some(transformed) = transformed.filter(|_| !fixed.is_empty()) {
        write!(stdout, "{}", SourceKindDiff::from_text(source, &transformed, Some(path)))?;
    }
    Ok(Diagnostics { inner: diagnostics, fixed: FixMap::from_iter([(relativize_path(path), fixed)]) })
}

// A fix that fails leaves the file as it was, so its diagnostics come from the original source, with nothing fixed. A
// source the fix loop already settled, whose withdrawn rules `settled` names, gets the loop's reports from one lint.
fn lint_source(path: &Path, source: &str, linter: &Linter, mode: &Mode, settled: Option<RuleSet>) -> Linted {
    let unfixed = || Linted {
        diagnostics: linter.lint(path, source, mode.noqa),
        transformed: None,
        fixed: FixTable::default(),
        settled: None,
    };
    if mode.fix.is_generate() {
        return unfixed();
    }
    if let Some(withdrawn) = settled {
        let mut linted = unfixed();
        Fixer::withdraw(&mut linted.diagnostics, withdrawn);
        return Linted { settled, ..linted };
    }
    let result = linter.fix(path, source, mode.unsafe_fixes, mode.noqa);
    let Ok((FixerResult { diagnostics, transformed, fixed }, withdrawn)) = result else { return unfixed() };
    let settled = (fixed.is_empty() && transformed == source).then_some(withdrawn);
    Linted { diagnostics, transformed: Some(transformed), fixed, settled }
}

// A source's reports, its text after the fixes when they ran, and the fixes applied; `settled` names the rules whose
// fixes the fix loop withdrew when it left the source as it was.
struct Linted {
    diagnostics: Vec<Diagnostic>,
    transformed: Option<String>,
    fixed: FixTable,
    settled: Option<RuleSet>,
}
