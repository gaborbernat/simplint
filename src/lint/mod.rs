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
use ruff_python_ast::PySourceType;
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::{ParseOptions, UnsupportedSyntaxError, parse_unchecked};
use ruff_source_file::SourceFileBuilder;
use ruff_text_size::TextSize;

use crate::lint::checker::Checker;
use crate::lint::noqa::Suppressions;
use crate::lint::ruff::RuffFile;
use crate::project::FileIndex;
use crate::settings::FileSettings;

pub struct Linter<'a> {
    pub settings: FileSettings<'a>,
    pub project: FileIndex<'a>,
    // Where the file sits in its package, such as `["package", "module"]`; `None` outside a package.
    pub module_path: Option<Vec<String>>,
    pub ruff: RuffFile<'a>,
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
        project: FileIndex<'a>,
        package: Option<PackageRoot<'a>>,
        module_path: Option<Vec<String>>,
    ) -> Self {
        Self { settings, project, module_path, ruff: RuffFile::new(&ruff.linter, package) }
    }

    // Like Ruff, a file that does not parse reports its syntax errors; the rules, which need a valid tree, skip it.
    pub fn lint(&self, path: &Path, source: &str, noqa: flags::Noqa) -> Lint {
        let (unsupported, outcome) = self.check(path, source, |mut diagnostics, suppressions| {
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

    // Ruff's `add_suppressions_to_path`: a comment for each report no comment suppresses yet. A file that does not parse
    // has no reports to suppress. Returns the number of lines changed and the new source.
    pub fn add_suppressions(
        &self,
        path: &Path,
        source: &str,
        kind: SuppressionKind,
        reason: Option<&str>,
    ) -> (usize, String) {
        let (_, outcome) = self.check(path, source, |diagnostics, suppressions| {
            suppressions.add(&suppressions.pending(diagnostics), kind, reason)
        });
        outcome.unwrap_or_else(|_| (0, source.to_owned()))
    }

    // Hands the rules' reports over a file that parses, with its suppression comments, to `then`; a file that does not
    // parse gives its syntax errors instead. The syntax errors of the target version come from the same parse.
    fn check<T>(
        &self,
        path: &Path,
        source: &str,
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
        let diagnostics = Checker::new(path, &file, &parsed, &indexer, &stylist, self).run();
        (unsupported, Ok(then(diagnostics, &Suppressions::new(path, source, &parsed, &indexer, &stylist))))
    }
}
