use std::collections::{HashMap, HashSet};
use std::ops::Deref;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use anyhow::{Result, ensure};
use ruff_db::diagnostic::Diagnostic;
use ruff_diagnostics::Applicability;
use ruff_linter::line_width::LineLength;
use ruff_linter::linter::{FixTable, ParseSource, lint_only, parse_unchecked_source};
use ruff_linter::package::PackageRoot;
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::LinterSettings;
use ruff_linter::settings::flags;
use ruff_linter::source_kind::SourceKind;
use ruff_python_ast::token::{Token, TokenKind, Tokens};
use ruff_python_ast::{ModModule, PySourceType, PythonVersion};
use ruff_python_parser::Parsed;
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::fix::apply::{FixResult, fix_ruff_file};
use crate::fix::errors;
use crate::fix::layout::{Layout, Widths};
use crate::settings::FromRuff;

// The file as Ruff sees it: the project's Ruff settings for it and the package it belongs to.
pub struct RuffFile<'a> {
    pub settings: &'a LinterSettings,
    package: Option<PackageRoot<'a>>,
    // The settings with no line length limit, made on first use, as most files need no Ruff run.
    unbounded: OnceLock<LinterSettings>,
}

impl<'a> RuffFile<'a> {
    pub fn new(settings: &'a LinterSettings, package: Option<PackageRoot<'a>>) -> Self {
        Self { settings, package, unbounded: OnceLock::new() }
    }

    // Ruff's report on the source at no line length limit. Layout is `ruff format`'s job, as it runs after simplint: a
    // finding about whitespace, blank lines or line length may come or go when it formats the code, so it does not
    // count either way, save a line too long for any layout.
    pub fn check(&self, path: &Path, source: &str, file: &FromRuff) -> Checked {
        const LAYOUT: [&str; 8] = ["E1", "E2", "E3", "W1", "W2", "W3", "E501", "W505"];
        let (source_kind, parsed) = Self::parse(source, file.parser_version);
        let valid_syntax = parsed.has_valid_syntax();
        let limit = usize::from(self.settings.pycodestyle.max_line_length.value());
        let widths = Widths { limit, line: file.line_width, indent: file.indent_width };
        let layout = Layout::of(source, parsed.tokens(), widths, file.formatted);
        let diagnostics = lint_only(
            path,
            self.package,
            self.unbounded(),
            flags::Noqa::Enabled,
            &source_kind,
            PySourceType::Python,
            ParseSource::Precomputed(parsed),
        )
        .diagnostics;
        let mut findings = Findings::default();
        for diagnostic in diagnostics.iter().filter(|diagnostic| {
            !diagnostic.is_invalid_syntax()
                && !diagnostic
                    .secondary_code()
                    .is_some_and(|code| LAYOUT.iter().any(|prefix| code.as_str().starts_with(prefix)))
        }) {
            let counts = findings.counts.entry(diagnostic.name()).or_default();
            counts.all += 1;
            let fixable = diagnostic.fix().is_some_and(|fix| fix.applies(Applicability::Unsafe));
            counts.fixable += usize::from(fixable);
            // An unsorted import block's finding is about the whole block: one that gained imports is new work, which
            // Ruff can take up once `ruff format` splits a statement sharing its last line. An unused suppression
            // left after `ruff check --fix` is one only the missing line length leaves unused, so it is not tracked.
            let name = diagnostic.name();
            if (fixable && name != RuffRule::UnusedNOQA.name().as_str())
                || name == RuffRule::UnsortedImports.name().as_str()
            {
                let lines = diagnostic.range().map_or("", |range| source.full_lines_str(range));
                findings.spans.insert((diagnostic.name(), lines.to_owned()));
            }
        }
        if layout.overlong > 0 && self.settings.rules.enabled(RuffRule::LineTooLong) {
            findings.counts.insert("line-too-long", Counts { all: layout.overlong, fixable: 0 });
        }
        if layout.moved_pragmas > 0 {
            findings.counts.insert("moved-pragma-comment", Counts { all: layout.moved_pragmas, fixable: 0 });
        }
        Checked { diagnostics, parses: valid_syntax, findings }
    }

    // The source as Ruff takes it, and its syntax tree.
    fn parse(source: &str, parser_version: PythonVersion) -> (SourceKind, Parsed<ModModule>) {
        let source_kind = SourceKind::Python { code: source.to_owned(), is_stub: false };
        let parsed = parse_unchecked_source(&source_kind, PySourceType::Python, parser_version);
        (source_kind, parsed)
    }

    // The rules with findings Ruff would fix at the project's line length. `ruff check --fix` ran at that length before
    // simplint, so a fix it offers yet left in place is one that fails, such as one that breaks the syntax.
    pub fn fixable_at_limit(&self, path: &Path, source: &str) -> HashSet<&'static str> {
        lint_only(
            path,
            self.package,
            self.settings,
            flags::Noqa::Enabled,
            &SourceKind::Python { code: source.to_owned(), is_stub: false },
            PySourceType::Python,
            ParseSource::None,
        )
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.fix().is_some_and(|fix| fix.applies(Applicability::Unsafe)))
        .map(Diagnostic::name)
        .collect()
    }

    // Ruff's `lint_fix` loop as `ruff check --fix --unsafe-fixes` runs it, applying only the fixes of the rules simplint's
    // fixes gave new findings. The other rules still report, as a suppression comment counts as unused when the rule
    // it silences is off, and its removal would expose what it silenced, such as an import kept for others. Every pass
    // is a `check`, which the caller remembers, so a pass over a source already checked costs nothing.
    pub fn cleanup(
        path: &Path,
        source: &str,
        introduced: &HashSet<&'static str>,
        parser_version: PythonVersion,
        check: impl Fn(&str) -> Arc<Checked>,
    ) -> String {
        // Ruff sorts an import block only when no other statement shares its last line, while `ruff format` puts each
        // statement after a `;` on a line of its own, so the import lines are split as the formatter would split them.
        let start = if introduced.contains(RuffRule::UnsortedImports.name().as_str()) {
            split_import_lines(source, Self::parse(source, parser_version).1.tokens())
        } else {
            source.to_owned()
        };
        Self::lint_fix(path, &start, introduced, check).unwrap_or(start)
    }

    // Ruff's `lint_fix` loop, failing as it does when a fix breaks code that parsed.
    fn lint_fix(
        path: &Path,
        start: &str,
        introduced: &HashSet<&'static str>,
        check: impl Fn(&str) -> Arc<Checked>,
    ) -> Result<String> {
        let mut checked = check(start);
        let valid = checked.parses && !checked.diagnostics.iter().any(Diagnostic::is_invalid_syntax);
        let mut transformed = start.to_owned();
        let mut fixed = FixTable::default();
        for iterations in 0.. {
            let offered = checked.diagnostics.iter().filter(|diagnostic| introduced.contains(diagnostic.name()));
            let converging = || !errors::ruff_failed_to_converge(path, iterations, &transformed, &checked.diagnostics);
            let Some(FixResult { code, fixes }) = fix_ruff_file(&transformed, offered).filter(|_| converging()) else {
                break;
            };
            for (id, code, count) in fixes.iter() {
                *fixed.entry(id).or_default(code) += count;
            }
            transformed = code;
            checked = check(&transformed);
            ensure!(!valid || checked.parses, errors::ruff_broken_syntax(path, &transformed, &fixed));
        }
        Ok(transformed)
    }

    // Ruff offers some fixes, such as collapsing nested `if` statements, only when the first line of their result fits
    // the line length, so whether it offers them would depend on the layout `ruff format` replaces. Without a limit
    // they depend on the code alone, and the layout model judges the lines they make. E501 then never fires, so its
    // code counts as external: unused-noqa (RUF100) would otherwise drop every `# noqa: E501`.
    fn unbounded(&self) -> &LinterSettings {
        self.unbounded.get_or_init(|| {
            let mut settings = self.settings.clone();
            settings.pycodestyle.max_line_length = LineLength::try_from(u16::MAX).expect("a nonzero length");
            settings.external.push("E501".to_owned());
            settings
        })
    }
}

// The source with each `;` that ends an import statement replaced by a line break at the statement's indentation.
fn split_import_lines(source: &str, tokens: &Tokens) -> String {
    let mut split = String::with_capacity(source.len());
    let mut copied = TextSize::default();
    let mut statement = None::<&Token>;
    let mut tokens = tokens.iter().filter(|token| !token.kind().is_trivia()).peekable();
    while let Some(token) = tokens.next() {
        match token.kind() {
            TokenKind::Semi => {
                if let (Some(start), Some(next)) = (statement.take(), tokens.peek())
                    && matches!(start.kind(), TokenKind::Import | TokenKind::From)
                {
                    split.push_str(&source[TextRange::new(copied, token.start())]);
                    split.push('\n');
                    split.push_str(leading_indentation(source.full_line_str(start.start())));
                    copied = next.start();
                }
            }
            TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent => statement = None,
            _ => {
                statement.get_or_insert(token);
            }
        }
    }
    split.push_str(&source[copied.to_usize()..]);
    split
}

// Ruff's diagnostics for a source, whether it parses, and the findings they make.
pub struct Checked {
    diagnostics: Vec<Diagnostic>,
    parses: bool,
    pub findings: Findings,
}

// Counted per rule, all findings and those `ruff check --fix --unsafe-fixes` would fix: a fix that edits code inside an
// existing finding, such as a rename, keeps its count. The fixable ones are also kept by rule and the lines they span:
// `ruff check --fix` already ran at the project's line length, so one it left is a fix offered only to a line that fits,
// such as merging nested `with` statements, and a fix that rewrites or reindents its lines can let it fit.
#[derive(Default)]
pub struct Findings {
    counts: HashMap<&'static str, Counts>,
    spans: HashSet<(&'static str, String)>,
}

#[derive(Clone, Copy, Default)]
struct Counts {
    all: usize,
    fixable: usize,
}

impl Findings {
    // The rules with more findings than before, or more that Ruff would fix, such as a return type it can now infer, or
    // a tracked one on lines that changed, unless its fix is one that fails.
    pub fn introduced_since<F: Deref<Target = HashSet<&'static str>>>(
        &self,
        before: &Self,
        failing: impl Fn() -> F,
    ) -> HashSet<&'static str> {
        let mut introduced: HashSet<&'static str> = self
            .counts
            .iter()
            .filter(|(name, counts)| {
                let counts_before = before.counts.get(*name).copied().unwrap_or_default();
                counts.all > counts_before.all || counts.fixable > counts_before.fixable
            })
            .map(|(name, _)| *name)
            .collect();
        // Which fixes fail takes a Ruff run of its own, needed only for findings on changed lines of rules not already
        // introduced.
        let moved: HashSet<&'static str> = self
            .spans
            .difference(&before.spans)
            .map(|(name, _)| *name)
            .filter(|name| !introduced.contains(name))
            .collect();
        if !moved.is_empty() {
            let failing = failing();
            introduced.extend(moved.into_iter().filter(|name| !failing.contains(name)));
        }
        introduced
    }
}
