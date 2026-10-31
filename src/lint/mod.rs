pub(crate) mod checker;
pub(crate) mod noqa;
pub(crate) mod registry;
pub(crate) mod walker;

use std::path::Path;

use anyhow::Result;
use ruff_db::diagnostic::{Diagnostic, Span};
use ruff_linter::SuppressionKind;
use ruff_linter::package::PackageRoot;
use ruff_linter::settings::flags;
use ruff_linter::settings::types::UnsafeFixes;
use ruff_python_ast::PySourceType;
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::parse_unchecked_source;
use ruff_source_file::SourceFileBuilder;

use crate::fix::ruff::RuffFile;
use crate::fix::{Fixer, FixerResult};
use crate::lint::checker::Checker;
use crate::lint::noqa::Suppressions;
use crate::lint::registry::RuleSet;
use crate::project::FileIndex;
use crate::settings::FileSettings;

pub struct Linter<'a> {
    pub settings: FileSettings<'a>,
    pub project: FileIndex<'a>,
    // Where the file sits in its package, such as `["package", "module"]`; `None` outside a package.
    pub module_path: Option<Vec<String>>,
    pub ruff: RuffFile<'a>,
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
    pub fn lint(&self, path: &Path, source: &str, noqa: flags::Noqa) -> Vec<Diagnostic> {
        self.check(path, source, |mut diagnostics, suppressions| {
            if noqa.is_enabled() {
                diagnostics = suppressions.unsuppressed(diagnostics);
            } else {
                suppressions.locate(&mut diagnostics);
            }
            diagnostics.sort_by(Diagnostic::ruff_start_ordering);
            diagnostics
        })
        .unwrap_or_else(|syntax_errors| syntax_errors)
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
        self.check(path, source, |diagnostics, suppressions| {
            suppressions.add(&suppressions.pending(diagnostics), kind, reason)
        })
        .unwrap_or_else(|_| (0, source.to_owned()))
    }

    // Hands the rules' reports over a file that parses, with its suppression comments, to `then`; a file that does not
    // parse gives its syntax errors instead.
    fn check<T>(
        &self,
        path: &Path,
        source: &str,
        then: impl FnOnce(Vec<Diagnostic>, &Suppressions) -> T,
    ) -> Result<T, Vec<Diagnostic>> {
        let file = SourceFileBuilder::new(path.to_string_lossy(), source).finish();
        let parsed = parse_unchecked_source(source, PySourceType::Python);
        if !parsed.has_valid_syntax() {
            return Err(parsed
                .errors()
                .iter()
                .map(|error| Diagnostic::invalid_syntax(Span::from(file.clone()), &error.error, error))
                .collect());
        }
        let indexer = Indexer::from_tokens(parsed.tokens(), source);
        let stylist = Stylist::from_tokens(parsed.tokens(), source);
        let diagnostics = Checker::new(path, &file, &parsed, &indexer, &stylist, self).run();
        Ok(then(diagnostics, &Suppressions::new(path, source, &parsed, &indexer, &stylist)))
    }

    // Fixes run after `ruff check --fix --unsafe-fixes`, so their result must not give it new work: a Ruff finding the
    // original file did not have gets Ruff's own fix when Ruff has one, and otherwise the fixes of the simplint rule that
    // caused it are withdrawn, keeping that rule's reports. `ruff format`, which runs last, keeps the syntax tree, so it
    // cannot undo this.
    pub fn fix(
        &self,
        path: &Path,
        source: &str,
        unsafe_fixes: UnsafeFixes,
        noqa: flags::Noqa,
    ) -> Result<(FixerResult, RuleSet)> {
        Fixer::new(self, path, unsafe_fixes, noqa).fix(source)
    }
}
