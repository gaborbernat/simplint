use std::cell::Cell;
use std::ops::Deref;
use std::path::Path;
use std::sync::OnceLock;

use memchr::memchr2_iter;
use ruff_db::diagnostic::Diagnostic;
use ruff_diagnostics::Applicability;
use ruff_linter::line_width::LineLength;
use ruff_linter::linter::{ParseSource, lint_only, parse_unchecked_source};
use ruff_linter::package::PackageRoot;
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::LinterSettings;
use ruff_linter::settings::flags;
use ruff_linter::source_kind::SourceKind;
use ruff_python_ast::{ModModule, PySourceType, PythonVersion};
use ruff_python_parser::Parsed;
use ruff_source_file::LineRanges;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::layout::{Layout, Widths};
use crate::settings::FromRuff;

thread_local! {
    static DIRECTIVE_WARNINGS: Cell<bool> = const { Cell::new(true) };
}

pub fn directive_warnings_shown() -> bool {
    DIRECTIVE_WARNINGS.get()
}

pub struct RuffFile<'a> {
    pub settings: &'a LinterSettings,
    package: Option<PackageRoot<'a>>,
    // The settings with no line length limit, made on first use, as most files need no Ruff run.
    unbounded: OnceLock<LinterSettings>,
    // Both settings without the layout rules, whose findings never count, so Ruff skips their checks.
    lean: OnceLock<LinterSettings>,
    lean_unbounded: OnceLock<LinterSettings>,
}

impl<'a> RuffFile<'a> {
    pub fn new(settings: &'a LinterSettings, package: Option<PackageRoot<'a>>) -> Self {
        Self { settings, package, unbounded: OnceLock::new(), lean: OnceLock::new(), lean_unbounded: OnceLock::new() }
    }

    // The directory of the package the file belongs to, which Ruff's own-package checks compare with.
    pub fn package(&self) -> Option<&'a Path> {
        self.package.map(PackageRoot::path)
    }

    // Ruff's report on the source at no line length limit. Layout is `ruff format`'s job, as it runs after simplint: a
    // finding about whitespace, blank lines or line length may come or go when it formats the code, so it does not
    // count either way, save a line too long for any layout.
    pub fn check(&self, path: &Path, source: &str, file: &FromRuff, directives_shown: bool) -> Checked {
        let (source_kind, parsed) = Self::parse(source, file.parser_version);
        let valid_syntax = parsed.has_valid_syntax();
        let limit = usize::from(self.settings.pycodestyle.max_line_length.value());
        let widths = Widths { limit, line: file.line_width, indent: file.indent_width };
        let layout = Layout::of(source, parsed.tokens(), widths, file.formatted);
        let diagnostics = with_directive_warnings(directives_shown, || {
            lint_only(
                path,
                self.package,
                Self::skipping_layout(source, self.unbounded(), &self.lean_unbounded),
                flags::Noqa::Enabled,
                &source_kind,
                PySourceType::Python,
                ParseSource::Precomputed(parsed),
            )
        })
        .diagnostics;
        let mut findings = Findings::default();
        for diagnostic in diagnostics.iter().filter(|diagnostic| {
            !diagnostic.is_invalid_syntax() && !diagnostic.secondary_code().is_some_and(|code| is_layout(code.as_str()))
        }) {
            let name = diagnostic.name();
            let counts = findings.counts.entry(name).or_default();
            counts.all += 1;
            let fixable = fixable_unsafely(diagnostic);
            counts.fixable += usize::from(fixable);
            // An unsorted import block's finding is about the whole block: one that gained imports is new work, which
            // Ruff can take up once `ruff format` splits a statement sharing its last line. An unused suppression
            // left after `ruff check --fix` is one only the missing line length leaves unused, so it is not tracked.
            if (fixable && name != RuffRule::UnusedNOQA.name().as_str())
                || name == RuffRule::UnsortedImports.name().as_str()
            {
                let lines = diagnostic.range().map_or("", |range| source.full_lines_str(range));
                findings.spans.insert((name, lines.to_owned()));
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

    pub fn parse(source: &str, parser_version: PythonVersion) -> (SourceKind, Parsed<ModModule>) {
        let source_kind = SourceKind::Python { code: source.to_owned(), is_stub: false };
        let parsed = parse_unchecked_source(&source_kind, PySourceType::Python, parser_version);
        (source_kind, parsed)
    }

    // The settings without the layout rules, unless a suppression comment may name one, as it counts as used only while
    // its rule runs: a blanket `noqa`, which any finding on its line uses, or a layout rule's code, or its name, which
    // `ruff: ignore[...]` and `ruff: disable[...]` take in preview.
    fn skipping_layout<'s>(
        source: &str,
        settings: &'s LinterSettings,
        lean: &'s OnceLock<LinterSettings>,
    ) -> &'s LinterSettings {
        let may_suppress_layout = has_blanket_noqa(source)
            || spells_layout_code(source)
            || source.contains("ruff")
                && settings
                    .rules
                    .iter_enabled()
                    .any(|rule| is_layout_rule(rule) && source.contains(rule.name().as_str()));
        if may_suppress_layout { settings } else { lean.get_or_init(|| without_layout(settings)) }
    }

    // The rules with findings Ruff would fix at the project's line length. `ruff check --fix` ran at that length before
    // simplint, so a fix it offers yet left in place is one that fails, such as one that breaks the syntax.
    pub fn fixable_at_limit(&self, path: &Path, source: &str) -> FxHashSet<&'static str> {
        with_directive_warnings(false, || {
            lint_only(
                path,
                self.package,
                Self::skipping_layout(source, self.settings, &self.lean),
                flags::Noqa::Enabled,
                &SourceKind::Python { code: source.to_owned(), is_stub: false },
                PySourceType::Python,
                ParseSource::None,
            )
        })
        .diagnostics
        .iter()
        .filter(|diagnostic| fixable_unsafely(diagnostic))
        .map(Diagnostic::name)
        .collect()
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

// Runs `lint` with the warnings Ruff logs about `noqa` directives shown or hidden. They name lines of the source Ruff
// lints, which only the source as read from disk has in common with the reports.
fn with_directive_warnings<T>(shown: bool, lint: impl FnOnce() -> T) -> T {
    let outer = DIRECTIVE_WARNINGS.replace(shown);
    let result = lint();
    DIRECTIVE_WARNINGS.set(outer);
    result
}

fn fixable_unsafely(diagnostic: &Diagnostic) -> bool {
    diagnostic.fix().is_some_and(|fix| fix.applies(Applicability::Unsafe))
}

// The rules about whitespace, blank lines and line length, by the prefixes of their codes: `ruff format` settles them.
fn is_layout(code: &str) -> bool {
    const LAYOUT: [&str; 8] = ["E1", "E2", "E3", "W1", "W2", "W3", "E501", "W505"];
    LAYOUT.iter().any(|prefix| code.starts_with(prefix))
}

fn is_layout_rule(rule: RuffRule) -> bool {
    rule.noqa_code().is_some_and(|code| is_layout(&code.to_string()))
}

fn without_layout(settings: &LinterSettings) -> LinterSettings {
    let mut lean = settings.clone();
    for rule in settings.rules.iter_enabled().filter(|rule| is_layout_rule(*rule)) {
        lean.rules.disable(rule);
    }
    lean
}

// Ruff reads `noqa` in any case, and as a blanket one unless a colon follows it.
fn has_blanket_noqa(source: &str) -> bool {
    let bytes = source.as_bytes();
    memchr2_iter(b'n', b'N', bytes).any(|start| {
        bytes.get(start..start + 4).is_some_and(|word| word.eq_ignore_ascii_case(b"noqa"))
            && !source[start + 4..]
                .trim_start_matches(|character: char| character.is_whitespace() && !matches!(character, '\n' | '\r'))
                .starts_with(':')
    })
}

// Suppressions match whole codes, and each layout rule's has three digits; a code may follow another without a
// separator, as in `noqa: F401E231`, but not a letter, which would make it part of a longer code or a word.
fn spells_layout_code(source: &str) -> bool {
    let bytes = source.as_bytes();
    memchr2_iter(b'E', b'W', bytes).any(|start| {
        !bytes[..start].last().is_some_and(u8::is_ascii_alphabetic)
            && bytes[start + 1..].iter().take_while(|byte| byte.is_ascii_digit()).count() == 3
            && is_layout(&source[start..start + 4])
    })
}

pub struct Checked {
    pub diagnostics: Vec<Diagnostic>,
    pub parses: bool,
    pub findings: Findings,
}

// Counted per rule, all findings and those `ruff check --fix --unsafe-fixes` would fix: a fix that edits code inside an
// existing finding, such as a rename, keeps its count. The fixable ones are also kept by rule and the lines they span:
// `ruff check --fix` already ran at the project's line length, so one it left is a fix offered only to a line that
// fits, such as merging nested `with` statements, and a fix that rewrites or reindents its lines can let it fit.
#[derive(Default)]
pub struct Findings {
    counts: FxHashMap<&'static str, Counts>,
    spans: FxHashSet<(&'static str, String)>,
}

#[derive(Clone, Copy, Default)]
struct Counts {
    all: usize,
    fixable: usize,
}

impl Findings {
    // The rules with more findings than before, or more that Ruff would fix, such as a return type it can now infer, or
    // a tracked one on lines that changed, unless its fix is one that fails.
    pub fn introduced_since<F: Deref<Target = FxHashSet<&'static str>>>(
        &self,
        before: &Self,
        failing: impl Fn() -> F,
    ) -> FxHashSet<&'static str> {
        let mut introduced: FxHashSet<&'static str> = self
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
        let moved: FxHashSet<&'static str> = self
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
