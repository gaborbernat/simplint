use std::borrow::Cow;
use std::cell::{OnceCell, RefCell};
use std::path::Path;

use ruff_db::diagnostic::{Annotation, Diagnostic, DiagnosticId, LintName, SecondaryCode, Severity, Span};
use ruff_diagnostics::Fix;
use ruff_python_ast::parenthesize::parenthesized_range;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, AnyNodeRef, Expr, ModModule, Stmt};
use ruff_python_codegen::Stylist;
use ruff_python_index::Indexer;
use ruff_python_parser::Parsed;
use ruff_python_trivia::{CommentRanges, has_leading_content};
use ruff_source_file::{LineRanges, SourceFile};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::lint::Linter;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::project::FileIndex;
use crate::rules::exports::{Exports, dunder_all};
use crate::rules::for_each_body;
use crate::rules::naming::string_values;
use crate::semantic::Semantic;
use crate::settings::FileSettings;

pub struct Checker<'a> {
    pub path: &'a Path,
    pub source: &'a str,
    pub parsed: &'a Parsed<ModModule>,
    pub settings: &'a FileSettings<'a>,
    pub stylist: &'a Stylist<'a>,
    file: &'a SourceFile,
    indexer: &'a Indexer,
    linter: &'a Linter<'a>,
    semantic: OnceCell<Semantic<'a>>,
    dunder_all: OnceCell<Exports<'a>>,
    function_defs: OnceCell<Vec<&'a ast::StmtFunctionDef>>,
    class_defs: OnceCell<Vec<&'a ast::StmtClassDef>>,
    name_tokens: OnceCell<FxHashMap<&'a str, usize>>,
    string_values: OnceCell<Vec<(TextRange, Cow<'a, str>)>>,
    // Rules report while they read the checker, as in Ruff's own `Checker`.
    diagnostics: RefCell<Vec<(Rule, Diagnostic)>>,
}

impl<'a> Checker<'a> {
    pub fn new(
        path: &'a Path,
        file: &'a SourceFile,
        parsed: &'a Parsed<ModModule>,
        indexer: &'a Indexer,
        stylist: &'a Stylist<'a>,
        linter: &'a Linter<'a>,
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
            semantic: OnceCell::new(),
            dunder_all: OnceCell::new(),
            function_defs: OnceCell::new(),
            class_defs: OnceCell::new(),
            name_tokens: OnceCell::new(),
            string_values: OnceCell::new(),
            diagnostics: RefCell::default(),
        }
    }

    pub fn run(self) -> Vec<Diagnostic> {
        let rules = self.settings.rules;
        for rule in rules.iter() {
            rule.check_module(&self);
        }
        if rules.iter().any(Rule::walks) {
            Walker::run(&self, rules);
        }
        let mut diagnostics = self.diagnostics.into_inner();
        // Reports at the same position keep the order of the rules, as when each rule ran alone; the sort is stable.
        diagnostics.sort_by_key(|(rule, _)| *rule);
        diagnostics.into_iter().map(|(_, diagnostic)| diagnostic).collect()
    }

    pub fn report(&self, rule: Rule, range: TextRange, message: impl Into<String>) {
        self.diagnostics.borrow_mut().push((rule, self.diagnostic(rule, range, message)));
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
        let mut diagnostic = self.diagnostic(rule, range, message);
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

    pub fn body(&self) -> &'a [Stmt] {
        &self.parsed.syntax().body
    }

    pub fn semantic(&self) -> &Semantic<'a> {
        self.semantic.get_or_init(|| Semantic::new(self.path, self.body(), self.settings.from_ruff.target_version))
    }

    pub fn text(&self, ranged: impl Ranged) -> &'a str {
        &self.source[ranged.range()]
    }

    pub fn parenthesized_text(&self, expr: &Expr, parent: AnyNodeRef) -> &'a str {
        let range = parenthesized_range(expr.into(), parent, self.comment_ranges(), self.source);
        &self.source[range.unwrap_or_else(|| expr.range())]
    }

    pub fn comment_ranges(&self) -> &'a CommentRanges {
        self.indexer.comment_ranges()
    }

    pub fn has_comment_in(&self, range: TextRange) -> bool {
        !self.comment_ranges().comments_in_range(range).is_empty()
    }

    // The comment after `node` on the line it ends on.
    pub fn trailing_comment(&self, node: impl Ranged) -> Option<TextRange> {
        let rest_of_line = TextRange::new(node.end(), self.source.line_end(node.end()));
        self.comment_ranges()
            .comments_in_range(rest_of_line)
            .iter()
            .copied()
            .find(|comment| comment.start() > node.end())
    }

    // The comment alone on the line right above `node`.
    pub fn comment_above(&self, node: impl Ranged) -> Option<TextRange> {
        let line_start = self.source.line_start(node.start());
        // On the first line, which may follow a byte order mark, there is no line above.
        let above = Some(line_start).filter(|start| *start > self.source.bom_start_offset())? - TextSize::from(1);
        let previous_line = TextRange::new(self.source.line_start(above), line_start);
        self.comment_ranges()
            .comments_in_range(previous_line)
            .first()
            .copied()
            .filter(|comment| !has_leading_content(comment.start(), self.source))
    }

    pub fn dunder_all(&self) -> &Exports<'a> {
        self.dunder_all.get_or_init(|| dunder_all(self.body()))
    }

    // Every `def` of the module, nested ones included.
    pub fn function_defs(&self) -> &[&'a ast::StmtFunctionDef] {
        self.function_defs.get_or_init(|| {
            let mut functions = Vec::new();
            for_each_body(self.body(), &mut |statements, _| {
                functions.extend(statements.iter().filter_map(Stmt::as_function_def_stmt));
            });
            functions
        })
    }

    // Every `class` of the module, nested ones included.
    pub fn class_defs(&self) -> &[&'a ast::StmtClassDef] {
        self.class_defs.get_or_init(|| {
            let mut classes = Vec::new();
            for_each_body(self.body(), &mut |statements, _| {
                classes.extend(statements.iter().filter_map(Stmt::as_class_def_stmt));
            });
            classes
        })
    }

    // How many identifier tokens spell `name`.
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

    // What each string of the module holds, with its range.
    pub fn string_values(&self) -> &[(TextRange, Cow<'a, str>)] {
        self.string_values.get_or_init(|| string_values(self.body()))
    }

    pub fn is_test_module(&self) -> bool {
        let name = self.file_name();
        name.starts_with("test_")
            || name.ends_with("_test.py")
            || name == "conftest.py"
            || self
                .path
                .strip_prefix(&self.settings.project.root)
                .unwrap_or(self.path)
                .components()
                .any(|part| matches!(part.as_os_str().to_str(), Some("tests" | "test")))
    }

    pub fn file_name(&self) -> &str {
        self.path.file_name().and_then(|name| name.to_str()).unwrap_or_default()
    }

    // What other files use from this one, and the attribute and base class names of the whole run.
    pub fn project(&self) -> FileIndex<'a> {
        self.linter.project
    }

    pub fn module_path(&self) -> Option<&'a [String]> {
        self.linter.module_path.as_deref()
    }

    pub fn is_init_module(&self) -> bool {
        self.file_name() == "__init__.py"
    }
}
