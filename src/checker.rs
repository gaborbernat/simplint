use std::cell::OnceCell;
use std::path::Path;

use ruff_diagnostics::Fix;
use ruff_python_ast::parenthesize::parenthesized_range;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{AnyNodeRef, Expr, ModModule, Stmt};
use ruff_python_codegen::Stylist;
use ruff_python_parser::Parsed;
use ruff_python_trivia::CommentRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::linter::Linter;
use crate::project::Project;
use crate::registry::Rule;
use crate::semantic::Semantic;
use crate::settings::Settings;

pub struct Diagnostic {
    pub rule: Rule,
    pub message: String,
    pub range: TextRange,
    pub fix: Option<Fix>,
}

pub struct Checker<'a> {
    pub path: &'a Path,
    pub source: &'a str,
    pub parsed: &'a Parsed<ModModule>,
    pub settings: &'a Settings,
    pub stylist: Stylist<'a>,
    linter: &'a Linter<'a>,
    semantic: OnceCell<Semantic<'a>>,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Checker<'a> {
    pub fn new(path: &'a Path, source: &'a str, parsed: &'a Parsed<ModModule>, linter: &'a Linter<'a>) -> Self {
        Self {
            path,
            source,
            parsed,
            settings: linter.settings,
            stylist: Stylist::from_tokens(parsed.tokens(), source),
            linter,
            semantic: OnceCell::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn run(mut self) -> Vec<Diagnostic> {
        for rule in self.linter.rules.iter() {
            rule.check(&mut self);
        }
        self.diagnostics
    }

    pub fn report(&mut self, rule: Rule, range: TextRange, message: impl Into<String>, fix: Option<Fix>) {
        self.diagnostics.push(Diagnostic { rule, message: message.into(), range, fix });
    }

    pub fn body(&self) -> &'a [Stmt] {
        &self.parsed.syntax().body
    }

    pub fn semantic(&self) -> &Semantic<'a> {
        self.semantic.get_or_init(|| Semantic::new(self.body()))
    }

    pub fn text(&self, ranged: impl Ranged) -> &'a str {
        &self.source[ranged.range()]
    }

    pub fn parenthesized_text(&self, expr: &Expr, parent: AnyNodeRef) -> &'a str {
        let comments = CommentRanges::new(self.comments().collect());
        &self.source[parenthesized_range(expr.into(), parent, &comments, self.source).unwrap_or_else(|| expr.range())]
    }

    pub fn comments(&self) -> impl Iterator<Item = TextRange> + 'a {
        self.parsed.tokens().iter().filter(|token| token.kind() == TokenKind::Comment).map(Ranged::range)
    }

    pub fn is_test_module(&self) -> bool {
        let name = self.file_name();
        name.starts_with("test_")
            || name.ends_with("_test.py")
            || name == "conftest.py"
            || self
                .path
                .strip_prefix(&self.settings.root)
                .unwrap_or(self.path)
                .components()
                .any(|part| matches!(part.as_os_str().to_str(), Some("tests" | "test")))
    }

    fn file_name(&self) -> &str {
        self.path.file_name().and_then(|name| name.to_str()).unwrap_or_default()
    }

    pub fn is_imported_elsewhere(&self, name: &str) -> bool {
        self.linter.project.is_imported(self.path, name)
    }

    pub fn project(&self) -> &'a Project {
        self.linter.project
    }

    pub fn is_init_module(&self) -> bool {
        self.file_name() == "__init__.py"
    }
}
