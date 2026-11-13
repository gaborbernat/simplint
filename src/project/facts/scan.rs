use std::collections::BTreeMap;
use std::collections::BTreeSet;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::TextRange;
use rustc_hash::FxHashSet;

use super::Parts;
use super::module_reads::{is_module_load, is_module_type};
use crate::project::computed_reads::ComputedReads;
use crate::project::constructors::discarded_call;
use crate::syntax::{attribute_call, identifier_words, is_prose_statement};

// One walk collects every fact that needs no scopes, since a walk per fact would visit every node again.
pub(super) struct Scan<'a> {
    parts: Parts,
    pub(super) attribute_loads: BTreeMap<String, u32>,
    pub(super) computed_reads: ComputedReads,
    pub(super) discarded: FxHashSet<TextRange>,
    pub(super) words: FxHashSet<String>,
    pub(super) global_names: BTreeSet<String>,
    pub(super) bound_marks: BTreeSet<String>,
    pub(super) modules: FxHashSet<String>,
    // Read once a body is walked, as the names it binds to modules may come later in the body.
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
            bound_marks: BTreeSet::new(),
            modules: FxHashSet::default(),
            returns: Vec::new(),
        };
        scan.visit_body(body);
        scan
    }
}

impl<'a> Visitor<'a> for Scan<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        // A docstring is prose, which names no patch target, so its words stay out of the run's words, unless a doctest
        // reads a name in it.
        if is_prose_statement(stmt) {
            return;
        }
        // `obj.name += 1` reads the attribute before it stores it.
        if let Stmt::AugAssign(ast::StmtAugAssign { target, .. }) = stmt
            && let Expr::Attribute(ast::ExprAttribute { attr, .. }) = target.as_ref()
        {
            *self.attribute_loads.entry(attr.to_string()).or_default() += 1;
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

impl<'a> Scan<'a> {
    // A word repeats across the strings, so it is copied once, when it first appears.
    fn add_words(&mut self, text: &str) {
        for word in identifier_words(text).filter(|word| !word.is_empty()) {
            if !self.words.contains(word) {
                self.words.insert(word.to_owned());
            }
        }
    }

    // A conftest that binds a mark to a name, as `C_EXT = pytest.mark.c_extension`, registers it through
    // `config.addinivalue_line("markers", f"{C_EXT.name}: ...")`, which spells no name in a string.
    fn add_bound_mark(&mut self, value: &Expr) {
        if self.parts.has(Parts::BOUND_MARKS)
            && let Expr::Attribute(ast::ExprAttribute { value: marks, attr, .. }) = value
            && UnqualifiedName::from_expr(marks).is_some_and(|name| name.segments().last() == Some(&"mark"))
        {
            self.bound_marks.insert(attr.to_string());
        }
    }

    // Before a statement's children, since a binding is read by the statements that follow it.
    fn open(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Expr(_) if self.parts.has(Parts::CALL_CONTEXTS) => self.discarded.extend(discarded_call(stmt)),
            Stmt::Global(ast::StmtGlobal { names, .. }) if self.parts.has(Parts::GLOBAL_NAMES) => {
                self.global_names.extend(names.iter().map(|name| name.as_str().to_owned()));
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                self.add_bound_mark(value);
                self.bind_module(targets, Some(value), None);
            }
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

    fn bind_module(&mut self, targets: &[Expr], value: Option<&Expr>, annotation: Option<&Expr>) {
        if value.is_some_and(|value| self.is_module(value)) || annotation.is_some_and(is_module_type) {
            self.modules.extend(targets.iter().filter_map(Expr::as_name_expr).map(|name| name.id.to_string()));
        }
    }

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
}

fn attribute_load(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Attribute(ast::ExprAttribute { attr, ctx, .. }) => {
            matches!(ctx, ExprContext::Load).then_some(attr.as_str())
        }
        _ => attribute_call(expr, &["getattr"])?.1.as_string_literal_expr().map(|name| name.value.to_str()),
    }
}

// The words a file's string literals hold, sorted and one per line, which the cache reads back in one allocation.
pub(super) fn string_words(words: FxHashSet<String>) -> String {
    let mut words: Vec<String> = words.into_iter().collect();
    words.sort_unstable();
    words.join("\n")
}
