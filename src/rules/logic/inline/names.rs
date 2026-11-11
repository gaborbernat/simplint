use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};
use ruff_python_parser::parse_expression;
use ruff_text_size::TextRange;
use rustc_hash::{FxHashMap, FxHashSet};

use super::declared_outer;
use crate::syntax::{alias_root_binding, last_segment, pattern_capture};

// Built-ins that read the locals by name, so a variable they read cannot be inlined away.
pub(in crate::rules::logic) const LOCALS_READERS: [&str; 4] = ["eval", "exec", "locals", "vars"];

// Every mention of each name in a function, found in one walk of its body, so a function with many assignments is
// walked once rather than once per assignment.
pub(in crate::rules::logic) struct Names<'a> {
    pub(in crate::rules::logic) mentions: FxHashMap<&'a str, Vec<(TextRange, ExprContext)>>,
    // Names the function binds in a way the walk does not follow: a `def`, a `class`, an import, an `except` target or
    // a pattern capture.
    pub(in crate::rules::logic) rebound: FxHashSet<&'a str>,
    pub(in crate::rules::logic) declared_outer: FxHashSet<&'a str>,
    // Names a string annotation or a `ForwardRef` reads. The walk does not see them, and they are read whenever the
    // annotation is evaluated, so no name they read is inlined.
    pub(in crate::rules::logic) string_reads: FxHashSet<String>,
    in_annotation: bool,
    // Whether the function reads its locals by name, which keeps every name alive.
    pub(in crate::rules::logic) reads_locals: bool,
}

impl<'a> Names<'a> {
    pub(in crate::rules::logic) fn of(body: &'a [Stmt]) -> Self {
        let mut names = Self {
            mentions: FxHashMap::default(),
            rebound: FxHashSet::default(),
            declared_outer: declared_outer(body),
            string_reads: FxHashSet::default(),
            in_annotation: false,
            reads_locals: false,
        };
        names.visit_body(body);
        names
    }
}

impl<'a> Visitor<'a> for Names<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(definition) => {
                self.rebound.insert(definition.name.as_str());
            }
            Stmt::ClassDef(class) => {
                self.rebound.insert(class.name.as_str());
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.rebound.extend(names.iter().map(alias_root_binding));
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name, .. }) = handler;
        self.rebound.extend(name.as_ref().map(ast::Identifier::as_str));
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        self.rebound.extend(pattern_capture(pattern).map(ast::Identifier::as_str));
        visitor::walk_pattern(self, pattern);
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        let outer = std::mem::replace(&mut self.in_annotation, true);
        visitor::walk_annotation(self, expr);
        self.in_annotation = outer;
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                self.mentions.entry(name.id.as_str()).or_default().push((name.range, name.ctx));
                self.reads_locals |= LOCALS_READERS.contains(&name.id.as_str());
            }
            Expr::StringLiteral(string) if self.in_annotation => self.read_forward_reference(string),
            Expr::Call(ast::ExprCall { func, arguments, .. }) if last_segment(func) == Some("ForwardRef") => {
                for argument in &arguments.args {
                    if let Expr::StringLiteral(string) = argument {
                        self.read_forward_reference(string);
                    }
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

impl Names<'_> {
    // A string annotation is code that Ruff reads as an expression, so the names it spells are reads.
    fn read_forward_reference(&mut self, string: &ast::ExprStringLiteral) {
        let Ok(parsed) = parse_expression(string.value.to_str()) else {
            return;
        };
        let mut collector = NameCollector::default();
        collector.visit_expr(parsed.expr());
        self.string_reads.extend(collector.names);
    }
}

// The names an expression reads, as their own text spells them.
#[derive(Default)]
struct NameCollector {
    names: Vec<String>,
}

impl Visitor<'_> for NameCollector {
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Name(name) = expr {
            self.names.push(name.id.to_string());
        }
        visitor::walk_expr(self, expr);
    }
}
