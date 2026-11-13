use std::borrow::Cow;
use std::cell::{Cell, OnceCell, RefCell};
use std::path::{Path, PathBuf};

use ruff_db::diagnostic::{Annotation, Diagnostic, DiagnosticId, LintName, SecondaryCode, Severity, Span};
use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::parenthesize::parenthesized_range;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::{self as ast, AnyNodeRef, Expr, ModModule, PythonVersion, Stmt};
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::Parsed;
use ruff_python_trivia::{CommentRanges, has_leading_content, has_trailing_content, leading_indentation};
use ruff_source_file::{LineRanges, OneIndexed, SourceFile, SourceFileBuilder};
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::lint::noqa::Suppressions;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::lint::{Linter, Parse};
use crate::project::FileIndex;
use crate::rules::FileState;
use crate::semantic::{RuffMatches, Semantic};
use crate::settings::FileSettings;
use crate::syntax::{Exports, dunder_all, for_each_body, previous_line_start, string_values};

pub struct Related {
    pub path: PathBuf,
    pub range: TextRange,
    pub message: String,
}

pub struct Checker<'a> {
    pub path: &'a Path,
    pub source: &'a str,
    pub parsed: &'a Parsed<ModModule>,
    pub settings: &'a FileSettings<'a>,
    pub stylist: &'a Stylist<'a>,
    file: &'a SourceFile,
    indexer: &'a Indexer,
    linter: &'a Linter<'a>,
    suppressions: &'a Suppressions<'a>,
    semantic: OnceCell<Semantic<'a>>,
    dunder_all: OnceCell<Exports<'a>>,
    defs: OnceCell<(Vec<&'a ast::StmtFunctionDef>, Vec<&'a ast::StmtClassDef>)>,
    module_classes: OnceCell<FxHashMap<&'a str, &'a ast::StmtClassDef>>,
    name_tokens: OnceCell<FxHashMap<&'a str, usize>>,
    string_values: OnceCell<Vec<(TextRange, Cow<'a, str>)>>,
    project: OnceCell<FileIndex<'a>>,
    // The other files that reports point into, read once each, so their lines are indexed once for rendering.
    related_files: RefCell<FxHashMap<PathBuf, Option<SourceFile>>>,
    state: FileState<'a>,
    // Rules report while they read the checker, as in Ruff's own `Checker`.
    diagnostics: RefCell<Vec<(Rule, Diagnostic)>>,
    // The rule whose code runs now, so a read of the other files of the run can be checked against its flag.
    rule: Cell<Option<Rule>>,
}

impl<'a> Checker<'a> {
    pub fn new(
        Parse { path, file, parsed, indexer, stylist }: Parse<'a>,
        linter: &'a Linter<'a>,
        suppressions: &'a Suppressions<'a>,
    ) -> Self {
        Self {
            path,
            source: file.source_text(),
            parsed,
            settings: &linter.settings,
            stylist,
            file,
            indexer,
            linter,
            suppressions,
            semantic: OnceCell::new(),
            dunder_all: OnceCell::new(),
            defs: OnceCell::new(),
            module_classes: OnceCell::new(),
            name_tokens: OnceCell::new(),
            string_values: OnceCell::new(),
            project: OnceCell::new(),
            related_files: RefCell::default(),
            state: FileState::default(),
            diagnostics: RefCell::default(),
            rule: Cell::new(None),
        }
    }

    pub fn run(self, unused_ignores_of: Option<&Suppressions>) -> Vec<Diagnostic> {
        let rules = self.settings.rules;
        for rule in rules.iter() {
            self.enter(rule);
            rule.check_module(&self);
        }
        self.rule.set(None);
        if rules.iter().any(Rule::walks) {
            Walker::run(&self, rules);
        }
        if let Some(suppressions) = unused_ignores_of
            && rules.contains(Rule::UnusedIgnore)
        {
            self.report_unused_ignores(suppressions);
        }
        let mut diagnostics = self.diagnostics.into_inner();
        // Reports at the same position keep the order of the rules, as when each rule ran alone; the sort is stable.
        diagnostics.sort_by_key(|(rule, _)| *rule);
        diagnostics.into_iter().map(|(_, diagnostic)| diagnostic).collect()
    }

    // The rule whose code runs next, which `assert_flagged_cross_file` checks the reads of.
    pub(super) fn enter(&self, rule: Rule) {
        self.rule.set(Some(rule));
    }

    fn report_unused_ignores(&self, suppressions: &Suppressions) {
        let diagnostics = self.diagnostics.borrow().iter().map(|(_, diagnostic)| diagnostic.clone()).collect();
        let Some(unused) = suppressions.unused_ignores(diagnostics, |proposed| {
            self.linter.lint_without_unused_ignore(self.path, proposed).diagnostics
        }) else {
            return;
        };
        let mut edits = unused.edits.into_iter().map(|(range, replacement)| {
            if replacement.is_empty() {
                Edit::range_deletion(range)
            } else {
                Edit::range_replacement(replacement, range)
            }
        });
        let first = edits.next().expect("an unused ignore has an edit");
        let range = TextRange::new(unused.start, self.source.line_end(unused.start));
        self.report_fix(
            Rule::UnusedIgnore,
            range,
            format!("{} unused simplint ignore selector{}", unused.count, if unused.count == 1 { "" } else { "s" }),
            Some(Fix::safe_edits(first, edits)),
            "Remove unused ignore selectors",
        );
    }

    // Ruff attaches a fix's title as a help note: the `help:` line of the full output, and the fix message of the JSON,
    // SARIF and GitHub formats. As in Ruff, the note stays when the settings make the rule unfixable.
    pub fn report_fix(
        &self,
        rule: Rule,
        range: TextRange,
        message: impl Into<String>,
        fix: Option<Fix>,
        title: impl Into<String>,
    ) {
        self.report_related(rule, range, message, fix, title, Vec::new());
    }

    // The locations a file cannot be read for are left out, since the report is still about its primary location.
    pub fn report_related(
        &self,
        rule: Rule,
        range: TextRange,
        message: impl Into<String>,
        fix: Option<Fix>,
        title: impl Into<String>,
        related: Vec<Related>,
    ) {
        let mut diagnostic = self.diagnostic(rule, range, message);
        let annotations = related.into_iter().filter_map(|location| {
            let file = if location.path == self.path {
                Some(self.file.clone())
            } else {
                let mut files = self.related_files.borrow_mut();
                files.entry(location.path).or_insert_with_key(|path| source_file_at(path)).clone()
            };
            file.map(|file| {
                Annotation::secondary(Span::from(file).with_range(location.range)).message(location.message)
            })
        });
        for annotation in annotations {
            diagnostic.annotate(annotation);
        }
        assert_disjoint_edits(rule, fix.as_ref());
        if fix.is_some() {
            diagnostic.help(title.into());
            diagnostic.set_optional_fix(self.settings.project.fix_for(rule, fix));
        }
        self.diagnostics.borrow_mut().push((rule, diagnostic));
    }

    // Built the way Ruff builds its own lint diagnostics, so every Ruff output format renders them.
    fn diagnostic(&self, rule: Rule, range: TextRange, message: impl Into<String>) -> Diagnostic {
        let mut diagnostic =
            Diagnostic::new(DiagnosticId::Lint(LintName::of(rule.name())), Severity::Error, message.into());
        diagnostic.annotate(Annotation::primary(Span::from(self.file.clone()).with_range(range)));
        diagnostic.set_secondary_code(SecondaryCode::new(rule.code().to_owned()));
        diagnostic.set_documentation_url(Some(rule.url()));
        diagnostic
    }

    pub fn report(&self, rule: Rule, range: TextRange, message: impl Into<String>) {
        self.diagnostics.borrow_mut().push((rule, self.diagnostic(rule, range, message)));
    }

    pub fn is_suppressed(&self, rule: Rule, range: TextRange) -> bool {
        self.suppressions.has_directives()
            && self.suppressions.pending(vec![self.diagnostic(rule, range, "")]).is_empty()
    }

    // Ruff renders a report's fix in full, so reports that share one fix offer it on the first report that no
    // suppression comment hides, and the fix is built only then.
    pub fn report_shared_fix(
        &self,
        rule: Rule,
        reports: Vec<(TextRange, String, String)>,
        fix: impl FnOnce() -> Option<Fix>,
    ) {
        let offer_at = self.first_unsuppressed(rule, reports.iter().map(|(range, ..)| *range));
        let mut fix = Some(fix);
        for (range, message, title) in reports {
            let carried = if offer_at == Some(range) { fix.take().and_then(|build| build()) } else { None };
            self.report_fix(rule, range, message, carried, title);
        }
    }

    fn first_unsuppressed(&self, rule: Rule, ranges: impl IntoIterator<Item = TextRange>) -> Option<TextRange> {
        let mut ranges = ranges.into_iter();
        let first = ranges.next()?;
        if !self.suppressions.has_directives() {
            return Some(first);
        }
        self.suppressions
            .pending(std::iter::once(first).chain(ranges).map(|range| self.diagnostic(rule, range, "")).collect())
            .first()
            .and_then(|(report, _)| report.range())
            // `--ignore-noqa` still needs the fix when every report has a suppression.
            .or(Some(first))
    }

    pub fn semantic(&self) -> &Semantic<'a> {
        self.semantic.get_or_init(|| {
            let matches = RuffMatches {
                list_appends: self.settings.rules.contains(Rule::CollectionBuiltByStatements),
                django_reports: self.settings.rules.contains(Rule::ClassMemberOrder),
                private_imports: self.settings.rules.contains(Rule::PrivateImportPath),
                unused_imports: self.is_init_module() && self.settings.rules.contains(Rule::InitModuleContent),
            };
            Semantic::new(
                self.path,
                self.body(),
                self.target_version(),
                self.linter.ruff.settings,
                self.linter.ruff.package(),
                matches,
            )
        })
    }

    pub fn is_init_module(&self) -> bool {
        self.file_name() == "__init__.py"
    }

    pub fn file_name(&self) -> &str {
        self.path.file_name().and_then(|name| name.to_str()).unwrap_or_default()
    }

    pub fn body(&self) -> &'a [Stmt] {
        &self.parsed.syntax().body
    }

    pub fn target_version(&self) -> PythonVersion {
        self.settings.from_ruff.target_version
    }

    pub fn parenthesized_text(&self, expr: &Expr, parent: AnyNodeRef) -> &'a str {
        &self.source[self.parenthesized_range(expr, parent)]
    }

    pub fn parenthesized_range(&self, expr: &Expr, parent: AnyNodeRef) -> TextRange {
        parenthesized_range(expr.into(), parent, self.comment_ranges(), self.source).unwrap_or_else(|| expr.range())
    }

    // The kind of the first token after `offset` that is neither a comment nor a line break inside brackets.
    pub fn next_significant_token(&self, offset: TextSize) -> Option<TokenKind> {
        self.parsed
            .tokens()
            .after(offset)
            .iter()
            .map(Token::kind)
            .find(|kind| !matches!(kind, TokenKind::NonLogicalNewline | TokenKind::Comment))
    }

    pub fn comment_ranges(&self) -> &'a CommentRanges {
        self.indexer.comment_ranges()
    }

    pub fn has_comment_in(&self, range: TextRange) -> bool {
        !self.comment_ranges().comments_in_range(range).is_empty()
    }

    pub fn trailing_comment(&self, node: impl Ranged) -> Option<TextRange> {
        let rest_of_line = TextRange::new(node.end(), self.source.line_end(node.end()));
        self.comment_ranges()
            .comments_in_range(rest_of_line)
            .iter()
            .copied()
            .find(|comment| comment.start() > node.end())
    }

    pub fn comment_above(&self, node: impl Ranged) -> Option<TextRange> {
        let line_start = self.source.line_start(node.start());
        // On the first line, which may follow a byte order mark, there is no line above.
        if line_start <= self.source.bom_start_offset() {
            return None;
        }
        let previous_line = TextRange::new(previous_line_start(self.source, line_start), line_start);
        self.comment_ranges()
            .comments_in_range(previous_line)
            .first()
            .copied()
            .filter(|comment| !has_leading_content(comment.start(), self.source))
    }

    pub fn is_alone_on_lines(&self, range: TextRange) -> bool {
        !has_leading_content(range.start(), self.source) && !has_trailing_content(range.end(), self.source)
    }

    // The leading whitespace of the line a node starts on; a node after a `;` shares the line, not its indentation.
    pub fn indentation(&self, ranged: impl Ranged) -> &'a str {
        leading_indentation(self.source.full_line_str(ranged.start()))
    }

    // Whether another file of the run reads the name from this module, or `__all__` lists it, so that removing its
    // binding would break a reader.
    pub fn is_exported(&self, name: &str) -> bool {
        self.project().is_referenced(name) || self.dunder_all().may_list(name)
    }

    pub fn project(&self) -> FileIndex<'a> {
        self.assert_flagged_cross_file();
        *self.project.get_or_init(|| self.linter.project.at(self.source))
    }

    // Whether the rule that runs reads what other files of the run contribute: the index of what they use from this
    // file and the attribute and base class names of the whole run, or the public API built from them. The cache keys a
    // file's result by that index only for rules `Rule::reads_other_files` flags, so an unflagged rule must not read
    // it. Nor may code outside a rule, whose result no flag covers.
    pub(crate) fn assert_flagged_cross_file(&self) {
        let rule = self.rule.get();
        let code = rule.map(Rule::code);
        debug_assert!(rule.is_some_and(Rule::reads_other_files), "{code:?} reads other files without being flagged");
    }

    pub fn dunder_all(&self) -> &Exports<'a> {
        self.dunder_all.get_or_init(|| dunder_all(self.body()))
    }

    // Every `def` of the module, nested ones included.
    pub fn function_defs(&self) -> &[&'a ast::StmtFunctionDef] {
        &self.defs().0
    }

    fn defs(&self) -> &(Vec<&'a ast::StmtFunctionDef>, Vec<&'a ast::StmtClassDef>) {
        self.defs.get_or_init(|| {
            let mut functions = Vec::new();
            let mut classes = Vec::new();
            for_each_body(self.body(), &mut |statements, _| {
                functions.extend(statements.iter().filter_map(Stmt::as_function_def_stmt));
                classes.extend(statements.iter().filter_map(Stmt::as_class_def_stmt));
            });
            (functions, classes)
        })
    }

    // Every `class` of the module, nested ones included.
    pub fn class_defs(&self) -> &[&'a ast::StmtClassDef] {
        &self.defs().1
    }

    // The classes of the module's top level by name, so that a name read as a class is found without a scan.
    pub fn module_classes(&self) -> &FxHashMap<&'a str, &'a ast::StmtClassDef> {
        self.module_classes.get_or_init(|| {
            self.body().iter().filter_map(Stmt::as_class_def_stmt).map(|class| (class.name.as_str(), class)).collect()
        })
    }

    pub fn name_token_count(&self, name: &str) -> usize {
        let counts = self.name_tokens.get_or_init(|| {
            let mut counts = FxHashMap::default();
            for token in self.parsed.tokens().iter().filter(|token| token.kind() == TokenKind::Identifier) {
                *counts.entry(self.text(token.range())).or_default() += 1;
            }
            counts
        });
        counts.get(name).copied().unwrap_or_default()
    }

    pub fn text(&self, ranged: impl Ranged) -> &'a str {
        &self.source[ranged.range()]
    }

    // An attribute such as `obj.name` reads another binding, so it does not count. Any other spelling, a parameter or
    // an import included, may bind the name.
    pub fn identifier_count_in(&self, range: TextRange, name: &str) -> usize {
        let tokens = self.parsed.tokens().in_range(range);
        tokens
            .iter()
            .enumerate()
            .filter(|(index, token)| {
                token.kind() == TokenKind::Identifier
                    && self.text(token.range()) == name
                    && index.checked_sub(1).is_none_or(|previous| tokens[previous].kind() != TokenKind::Dot)
            })
            .count()
    }

    pub fn enclosing_scope(&self, range: TextRange) -> TextRange {
        let scopes = self.state().scope_nesting(self);
        scopes.holders(range).next().map_or_else(|| self.file_range(), |index| scopes.range(index))
    }

    pub(crate) fn state(&self) -> &FileState<'a> {
        &self.state
    }

    pub fn file_range(&self) -> TextRange {
        TextRange::up_to(self.source.text_len())
    }

    pub fn string_values(&self) -> &[(TextRange, Cow<'a, str>)] {
        self.string_values.get_or_init(|| string_values(self.body()))
    }

    pub fn is_test_module(&self) -> bool {
        self.settings.is_test
    }

    pub fn line_ending(&self) -> &'static str {
        self.stylist.line_ending().as_str()
    }

    pub fn module_path(&self) -> Option<&'a [String]> {
        self.linter.module_path.as_deref()
    }

    pub fn has_sibling_init(&self) -> bool {
        self.linter.sibling_init
    }

    // The source file indexes its lines once, for every report that names one and for rendering the diagnostics.
    pub fn line_number(&self, offset: TextSize) -> OneIndexed {
        self.file.index().line_index(offset)
    }
}

// The source of a file of the run, read for a location in it, under the name the run gives the file.
pub(crate) fn source_file_at(path: &Path) -> Option<SourceFile> {
    std::fs::read_to_string(path).ok().map(|source| SourceFileBuilder::new(path.to_string_lossy(), source).finish())
}

// Edits of a fix are sorted and disjoint as `Fix` requires. Touching edits, such as an insertion at the end of a
// deletion, do not overlap.
fn disjoint_edits(fix: &Fix) -> bool {
    fix.edits().windows(2).all(|pair| pair[1].start() >= pair[0].end())
}

// Applying a fix and rendering its diff both walk its edits in order, which overlapping edits break.
fn assert_disjoint_edits(rule: Rule, fix: Option<&Fix>) {
    debug_assert!(fix.is_none_or(disjoint_edits), "{} built a fix with overlapping edits", rule.name());
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::{Edit, Fix};
    use ruff_text_size::TextSize;

    use super::assert_disjoint_edits;
    use crate::lint::registry::Rule;

    #[test]
    fn touching_edits_are_disjoint() {
        let fix = Fix::safe_edits(
            Edit::deletion(TextSize::new(0), TextSize::new(2)),
            [Edit::insertion("x".to_owned(), TextSize::new(2))],
        );

        assert_disjoint_edits(Rule::UnusedIgnore, Some(&fix));
    }

    #[test]
    #[should_panic(expected = "unused-ignore built a fix with overlapping edits")]
    fn overlapping_edits_fail_the_check() {
        let fix = Fix::safe_edits(
            Edit::deletion(TextSize::new(0), TextSize::new(3)),
            [Edit::deletion(TextSize::new(2), TextSize::new(4))],
        );

        assert_disjoint_edits(Rule::UnusedIgnore, Some(&fix));
    }
}
