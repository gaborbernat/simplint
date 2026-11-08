use std::collections::BTreeMap;
use std::collections::BTreeSet;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use super::Parts;
use super::module_reads::{is_module_load, is_module_type};
use crate::project::computed_reads::ComputedReads;
use crate::syntax::{attribute_call, is_prose_statement};

// The facts that need no scopes, collected in one walk of the module rather than a walk for each, which would visit
// every node again.
pub(super) struct Scan<'a> {
    parts: Parts,
    // How many times the module loads each attribute name, as `obj.name` or `getattr(obj, "name")`.
    pub(super) attribute_loads: BTreeMap<String, u32>,
    pub(super) computed_reads: ComputedReads,
    // The calls whose value a statement drops, as `f()` on a line of its own.
    pub(super) discarded: FxHashSet<TextRange>,
    // The words the string literals hold, as the words a name spelled in a string is.
    pub(super) words: FxHashSet<String>,
    // The names a `global` statement declares.
    pub(super) global_names: BTreeSet<String>,
    // The names bound to module objects, wherever the file binds them.
    pub(super) modules: FxHashSet<String>,
    // The values each enclosing function or class returns, innermost last; a function's returns are read once its body
    // is walked, as the names it binds to modules may come later in the body.
    returns: Vec<Vec<&'a Expr>>,
}

impl<'a> Scan<'a> {
    pub(super) fn of(body: &'a [Stmt], parts: Parts) -> Self {
        let mut scan = Self {
            parts,
            attribute_loads: BTreeMap::new(),
            computed_reads: ComputedReads::default(),
            discarded: FxHashSet::default(),
            words: FxHashSet::default(),
            global_names: BTreeSet::new(),
            modules: FxHashSet::default(),
            returns: Vec::new(),
        };
        scan.visit_body(body);
        scan
    }

    // A word repeats across the strings, so it is copied once, when it first appears.
    fn add_words(&mut self, text: &str) {
        for word in text.split(|char: char| !(char.is_alphanumeric() || char == '_')).filter(|word| !word.is_empty()) {
            if !self.words.contains(word) {
                self.words.insert(word.to_owned());
            }
        }
    }

    // Before a statement's children, since a binding is read by the statements that follow it.
    fn open(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Expr(ast::StmtExpr { value, .. })
                if self.parts.has(Parts::CALL_CONTEXTS)
                    && let Expr::Call(call) = value.as_ref() =>
            {
                self.discarded.insert(call.range());
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) if self.parts.has(Parts::GLOBAL_NAMES) => {
                self.global_names.extend(names.iter().map(|name| name.as_str().to_owned()));
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => self.bind_module(targets, Some(value), None),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value, annotation, .. }) => {
                self.bind_module(std::slice::from_ref(target.as_ref()), value.as_deref(), Some(annotation));
            }
            Stmt::Return(ast::StmtReturn { value: Some(value), .. }) => {
                if let Some(returns) = self.returns.last_mut() {
                    returns.push(value);
                }
            }
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => self.returns.push(Vec::new()),
            _ => {}
        }
    }

    // After a statement's children, so the returns of a function are all known. A pytest fixture that returns a module
    // hands it to each test under the fixture's name, which a decorator may give.
    fn close(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                let returns = self.returns.pop().unwrap_or_default();
                if returns.into_iter().any(|value| self.is_module(value)) {
                    self.modules.insert(function.name.to_string());
                    self.modules.extend(
                        function
                            .decorator_list
                            .iter()
                            .filter_map(|decorator| decorator.expression.as_call_expr()?.arguments.find_keyword("name"))
                            .filter_map(|keyword| {
                                Some(keyword.value.as_string_literal_expr()?.value.to_str().to_owned())
                            }),
                    );
                }
            }
            Stmt::ClassDef(_) => _ = self.returns.pop(),
            _ => {}
        }
    }

    // A name bound to a module load, a name already bound to a module, or an annotation of `ModuleType`.
    fn bind_module(&mut self, targets: &[Expr], value: Option<&Expr>, annotation: Option<&Expr>) {
        if value.is_some_and(|value| self.is_module(value)) || annotation.is_some_and(is_module_type) {
            self.modules.extend(targets.iter().filter_map(Expr::as_name_expr).map(|name| name.id.to_string()));
        }
    }

    // A module load, or a name already bound to a module, as is or through `typing.cast`.
    fn is_module(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => self.modules.contains(name.id.as_str()),
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().last() == Some(&"cast")) =>
            {
                arguments.args.get(1).is_some_and(|value| self.is_module(value))
            }
            _ => is_module_load(expr),
        }
    }
}

impl<'a> Visitor<'a> for Scan<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        // A docstring is prose, which names no patch target, so its words stay out of the run's words, unless a doctest
        // reads a name in it.
        if is_prose_statement(stmt) {
            return;
        }
        self.open(stmt);
        visitor::walk_stmt(self, stmt);
        self.close(stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(name) = attribute_load(expr) {
            match self.attribute_loads.get_mut(name) {
                Some(count) => *count += 1,
                None => _ = self.attribute_loads.insert(name.to_owned(), 1),
            }
        }
        if self.parts.has(Parts::COMPUTED_READS)
            && let Some(read) = ComputedReads::read(expr)
        {
            self.computed_reads.insert(read);
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_parameter(&mut self, parameter: &'a ast::Parameter) {
        if parameter.annotation.as_deref().is_some_and(is_module_type) {
            self.modules.insert(parameter.name.to_string());
        }
        visitor::walk_parameter(self, parameter);
    }

    fn visit_string_literal(&mut self, literal: &'a ast::StringLiteral) {
        if self.parts.has(Parts::STRING_WORDS) {
            self.add_words(&literal.value);
        }
    }

    fn visit_bytes_literal(&mut self, literal: &'a ast::BytesLiteral) {
        if self.parts.has(Parts::STRING_WORDS) {
            self.add_words(&String::from_utf8_lossy(&literal.value));
        }
    }

    fn visit_interpolated_string_element(&mut self, element: &'a ast::InterpolatedStringElement) {
        if let ast::InterpolatedStringElement::Literal(literal) = element
            && self.parts.has(Parts::STRING_WORDS)
        {
            self.add_words(&literal.value);
        }
        visitor::walk_interpolated_string_element(self, element);
    }
}

// The attribute an expression loads: `name` of `obj.name` in a load context, or of `getattr(obj, "name")`.
fn attribute_load(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Attribute(ast::ExprAttribute { attr, ctx, .. }) => {
            matches!(ctx, ExprContext::Load).then_some(attr.as_str())
        }
        _ => attribute_call(expr, &["getattr"])?.1.as_string_literal_expr().map(|name| name.value.to_str()),
    }
}
