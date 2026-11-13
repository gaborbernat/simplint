pub(crate) mod checker;
pub(crate) mod examples;
pub(crate) mod noqa;
pub(crate) mod registry;
pub(crate) mod ruff;
pub(crate) mod walker;

use std::path::Path;

use anyhow::Result;
use ruff_db::diagnostic::{Diagnostic, Span};
use ruff_linter::SuppressionKind;
use ruff_linter::package::PackageRoot;
use ruff_linter::settings::flags;
use ruff_python_ast::{ModModule, PySourceType};
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::{ParseOptions, Parsed, UnsupportedSyntaxError, parse_unchecked};
use ruff_source_file::{SourceFile, SourceFileBuilder};
use ruff_text_size::TextSize;

use crate::lint::checker::Checker;
use crate::lint::noqa::Suppressions;
use crate::lint::ruff::RuffFile;
use crate::project::FileIndex;
use crate::settings::FileSettings;

pub struct Linter<'a> {
    pub settings: FileSettings<'a>,
    pub project: FileIndex<'a>,
    pub module_path: Option<Vec<String>>,
    pub ruff: RuffFile<'a>,
    // Whether the file's directory holds an `__init__.py`, when a rule reads it; the run checks only then.
    pub sibling_init: bool,
}

// One parse of a file's source, with the indexes built from its tokens.
#[derive(Clone, Copy)]
pub struct Parse<'a> {
    pub path: &'a Path,
    pub file: &'a SourceFile,
    pub parsed: &'a Parsed<ModModule>,
    pub indexer: &'a Indexer,
    pub stylist: &'a Stylist<'a>,
}

// A file's reports and the syntax errors its target version rejects, both from one parse of its source. With `noqa`
// comments applying, `noqa_rows` holds the line each report's suppression comment belongs on, in the order of
// `diagnostics`, which keeps it once the diagnostic is stored.
#[derive(Clone)]
pub struct Lint {
    pub diagnostics: Vec<Diagnostic>,
    pub unsupported: Vec<UnsupportedSyntaxError>,
    pub noqa_rows: Vec<TextSize>,
}

impl<'a> Linter<'a> {
    pub fn new(
        ruff: &'a ruff_workspace::Settings,
        settings: FileSettings<'a>,
        project: &FileIndex<'a>,
        package: Option<PackageRoot<'a>>,
        module_path: Option<Vec<String>>,
        sibling_init: bool,
    ) -> Self {
        Self { settings, project: *project, module_path, ruff: RuffFile::new(&ruff.linter, package), sibling_init }
    }

    // The syntax errors of a file that does not parse come back for the fix loop, which reads them to catch a fix that
    // breaks the syntax; a run leaves them out, as Ruff reports them.
    pub fn lint(&self, path: &Path, source: &str, noqa: flags::Noqa) -> Lint {
        self.lint_inner(path, source, noqa, noqa.is_enabled())
    }

    fn lint_inner(&self, path: &Path, source: &str, noqa: flags::Noqa, include_unused_ignore: bool) -> Lint {
        let (unsupported, outcome) =
            self.check(path, source, include_unused_ignore, |mut diagnostics, suppressions| {
                diagnostics.sort_by(Diagnostic::ruff_start_ordering);
                if noqa.is_enabled() {
                    suppressions.pending(diagnostics).into_iter().unzip()
                } else {
                    let noqa_rows = suppressions.locate(&mut diagnostics);
                    (diagnostics, noqa_rows)
                }
            });
        match outcome {
            Ok((diagnostics, noqa_rows)) => Lint { diagnostics, unsupported, noqa_rows },
            Err(syntax_errors) => Lint { diagnostics: syntax_errors, unsupported, noqa_rows: Vec::new() },
        }
    }

    // The syntax errors of the target version come from the same parse.
    fn check<T>(
        &self,
        path: &Path,
        source: &str,
        include_unused_ignore: bool,
        then: impl FnOnce(Vec<Diagnostic>, &Suppressions) -> T,
    ) -> (Vec<UnsupportedSyntaxError>, Result<T, Vec<Diagnostic>>) {
        let file = SourceFileBuilder::new(path.to_string_lossy(), source).finish();
        let options =
            ParseOptions::from(PySourceType::Python).with_target_version(self.settings.from_ruff.target_version);
        // A Python source parses to a module, whatever the target version.
        let parsed = parse_unchecked(source, options).try_into_module().expect("a Python source parses to a module");
        let unsupported = parsed.unsupported_syntax_errors().to_vec();
        if !parsed.has_valid_syntax() {
            let errors = parsed
                .errors()
                .iter()
                .map(|error| Diagnostic::invalid_syntax(Span::from(file.clone()), &error.error, error))
                .collect();
            return (unsupported, Err(errors));
        }
        let indexer = Indexer::from_tokens(parsed.tokens(), source);
        let stylist = Stylist::from_tokens(parsed.tokens(), source);
        let parse = Parse { path, file: &file, parsed: &parsed, indexer: &indexer, stylist: &stylist };
        let suppressions = Suppressions::new(parse);
        let diagnostics = Checker::new(parse, self, &suppressions).run(include_unused_ignore.then_some(&suppressions));
        (unsupported, Ok(then(diagnostics, &suppressions)))
    }

    pub(crate) fn lint_without_unused_ignore(&self, path: &Path, source: &str) -> Lint {
        self.lint_inner(path, source, flags::Noqa::Enabled, false)
    }

    // Ruff's `add_suppressions_to_path`: a comment for each report no comment suppresses yet. A file that does not
    // parse has no reports to suppress.
    pub fn add_suppressions(
        &self,
        path: &Path,
        source: &str,
        kind: SuppressionKind,
        reason: Option<&str>,
    ) -> (usize, String) {
        let (_, outcome) = self.check(path, source, false, |diagnostics, suppressions| {
            suppressions.add(&suppressions.pending(diagnostics), kind, reason)
        });
        outcome.unwrap_or_else(|_| (0, source.to_owned()))
    }
}

pub(crate) fn relative_path(path: &Path) -> String {
    let relative = std::env::current_dir().ok().and_then(|cwd| path.strip_prefix(cwd).ok());
    relative.unwrap_or(path).display().to_string()
}
