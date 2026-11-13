use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::flow::has_own_break;
use crate::rules::logic::name_uses::{LocalScope, comprehension_generators};

// The builtins that return a one-pass iterator.
const ITERATOR_BUILTINS: [&str; 5] = ["map", "filter", "zip", "reversed", "enumerate"];
// The builtins that read an iterable to its end.
const CONSUMERS: [&str; 12] =
    ["list", "tuple", "set", "frozenset", "sorted", "sum", "min", "max", "any", "all", "dict", "len"];

pub fn iterator_reuse(checker: &Checker) {
    for scope in checker.state().local_scopes(checker).iter() {
        check_scope(checker, scope);
    }
}

fn is_iterator_annotation(checker: &Checker, annotation: &Expr) -> bool {
    let base = match annotation {
        Expr::Subscript(subscript) => subscript.value.as_ref(),
        _ => annotation,
    };
    let semantic = checker.semantic();
    ["Iterator", "Generator"]
        .into_iter()
        .any(|name| semantic.match_typing_expr(base, name) || semantic.resolves_to(base, &["collections", "abc", name]))
}

fn check_scope<'a>(checker: &Checker<'a>, scope: &LocalScope<'a>) {
    let mut walk = Walk { checker, bindings: Vec::new(), sites: Vec::new(), path: Vec::new() };
    walk.visit_body(scope.body);
    let parameters = scope.function.into_iter().flat_map(|function| function.parameters.iter()).filter(|parameter| {
        parameter.annotation().is_some_and(|annotation| is_iterator_annotation(checker, annotation))
    });
    let mut bindings =
        parameters.map(|parameter| (parameter.name().as_str(), Vec::new())).chain(walk.bindings).peekable();
    if bindings.peek().is_none() {
        return;
    }
    let uses = scope.uses();
    let candidates: FxHashMap<&str, Vec<Branch>> = bindings.filter(|(name, _)| uses.is_single_binding(name)).collect();
    if candidates.is_empty() {
        return;
    }
    let mut sites_by_name: FxHashMap<&str, Vec<&Site>> = FxHashMap::default();
    for site in walk.sites.iter().filter(|site| candidates.contains_key(site.name)) {
        sites_by_name.entry(site.name).or_default().push(site);
    }
    for (name, binding_path) in &candidates {
        let sites = sites_by_name.remove(name).unwrap_or_default();
        // Any other read hands the iterator to code that may advance it, as `next()` does on purpose.
        if uses.loads(name).len() != sites.len() || sites.iter().any(|site| site.resumable) {
            continue;
        }
        report(checker, name, binding_path, &sites);
    }
}

fn report(checker: &Checker, name: &str, binding_path: &[Branch], sites: &[&Site]) {
    for (index, site) in sites.iter().enumerate() {
        if site.path.iter().any(|branch| branch.is_loop && !binding_path.contains(branch)) {
            checker.report(
                Rule::IteratorReuse,
                site.range,
                format!("`{name}` is a one-pass iterator read inside a loop; every pass after the first sees it empty"),
            );
            continue;
        }
        if sites[..index].iter().any(|earlier| !exclusive(&earlier.path, &site.path)) {
            checker.report(
                Rule::IteratorReuse,
                site.range,
                format!("`{name}` is a one-pass iterator that an earlier read already used up, so this sees it empty"),
            );
        }
    }
}

// Two reads in different arms of one `if`, `match` or `try`, or after an arm that leaves the block, never both run.
fn exclusive(earlier: &[Branch], later: &[Branch]) -> bool {
    for (index, branch) in earlier.iter().enumerate() {
        match later.get(index) {
            Some(other) if other == branch => {}
            Some(other) if other.statement == branch.statement => return true,
            _ => return earlier[index..].iter().any(|branch| branch.exits),
        }
    }
    false
}

#[derive(Clone, PartialEq, Eq)]
struct Branch {
    statement: usize,
    arm: usize,
    is_loop: bool,
    // The arm ends with `return`, `raise`, `break` or `continue`.
    exits: bool,
}

struct Site<'a> {
    name: &'a str,
    range: TextRange,
    path: Vec<Branch>,
    // A `for` loop that can `break` out early, so a later loop resumes where it stopped.
    resumable: bool,
}

// One walk of a scope numbers its branches and records both the names bound to a one-pass iterator, with the branch
// path of the binding, and every read that consumes a name, with the branch path of the read.
struct Walk<'c, 'a> {
    checker: &'c Checker<'a>,
    bindings: Vec<(&'a str, Vec<Branch>)>,
    sites: Vec<Site<'a>>,
    path: Vec<Branch>,
}

impl<'a> Walk<'_, 'a> {
    fn is_iterator_value(&self, value: &Expr) -> bool {
        match value {
            Expr::Generator(_) => true,
            Expr::Call(call) => {
                ITERATOR_BUILTINS.iter().any(|name| self.checker.semantic().is_builtin(&call.func, name))
            }
            _ => false,
        }
    }

    fn site(&mut self, expr: &'a Expr, resumable: bool) {
        if let Expr::Name(name) = expr {
            self.sites.push(Site { name: name.id.as_str(), range: name.range, path: self.path.clone(), resumable });
        }
    }

    fn arm<T>(&mut self, statement: usize, arm: usize, is_loop: bool, body: &'a [Stmt], visit: T)
    where
        T: FnOnce(&mut Self, &'a [Stmt]),
    {
        self.path.push(Branch { statement, arm, is_loop, exits: exits(body) });
        visit(self, body);
        self.path.pop();
    }
}

fn exits(body: &[Stmt]) -> bool {
    matches!(body.last(), Some(Stmt::Return(_) | Stmt::Raise(_) | Stmt::Break(_) | Stmt::Continue(_)))
}

impl<'a> Visitor<'a> for Walk<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                if let [Expr::Name(target)] = targets.as_slice()
                    && self.is_iterator_value(value)
                {
                    self.bindings.push((target.id.as_str(), self.path.clone()));
                }
                walk_stmt(self, stmt);
            }
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) => {
                self.site(iter, has_own_break(body));
                self.visit_expr(iter);
                self.visit_expr(target);
                let id = stmt.start().to_usize();
                self.arm(id, 0, true, body, ruff_python_ast::visitor::Visitor::visit_body);
                self.visit_body(orelse);
            }
            Stmt::While(ast::StmtWhile { test, body, orelse, .. }) => {
                let id = stmt.start().to_usize();
                self.path.push(Branch { statement: id, arm: 0, is_loop: true, exits: exits(body) });
                self.visit_expr(test);
                self.visit_body(body);
                self.path.pop();
                self.visit_body(orelse);
            }
            Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) => {
                self.visit_expr(test);
                let id = stmt.start().to_usize();
                self.arm(id, 0, false, body, ruff_python_ast::visitor::Visitor::visit_body);
                for (index, clause) in elif_else_clauses.iter().enumerate() {
                    if let Some(test) = &clause.test {
                        self.visit_expr(test);
                    }
                    self.arm(id, index + 1, false, &clause.body, ruff_python_ast::visitor::Visitor::visit_body);
                }
            }
            Stmt::Match(ast::StmtMatch { subject, cases, .. }) => {
                self.visit_expr(subject);
                let id = stmt.start().to_usize();
                for (index, case) in cases.iter().enumerate() {
                    self.arm(id, index, false, &case.body, ruff_python_ast::visitor::Visitor::visit_body);
                }
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                let id = stmt.start().to_usize();
                self.arm(id, 0, false, body, ruff_python_ast::visitor::Visitor::visit_body);
                for (index, handler) in handlers.iter().enumerate() {
                    let ast::ExceptHandler::ExceptHandler(handler) = handler;
                    self.arm(id, index + 1, false, &handler.body, ruff_python_ast::visitor::Visitor::visit_body);
                }
                self.visit_body(orelse);
                self.visit_body(finalbody);
            }
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => {}
            Expr::Call(call) => {
                let consumes =
                    CONSUMERS.iter().any(|name| self.checker.semantic().is_builtin(&call.func, name)) || is_join(call);
                if consumes && let [argument] = call.arguments.args.as_ref() {
                    self.site(argument, false);
                }
                walk_expr(self, expr);
            }
            Expr::Starred(ast::ExprStarred { value, .. }) => {
                self.site(value, false);
                walk_expr(self, expr);
            }
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::Generator(_) | Expr::DictComp(_) => {
                let generators = comprehension_generators(expr);
                // The first iterable is read once, where the comprehension runs; the rest run once per item.
                if let Some(first) = generators.first() {
                    self.site(&first.iter, false);
                    self.visit_expr(&first.iter);
                }
                let id = expr.start().to_usize();
                self.path.push(Branch { statement: id, arm: 0, is_loop: true, exits: false });
                for (index, generator) in generators.iter().enumerate() {
                    if index > 0 {
                        self.site(&generator.iter, false);
                        self.visit_expr(&generator.iter);
                    }
                    for condition in &generator.ifs {
                        self.visit_expr(condition);
                    }
                }
                match expr {
                    Expr::ListComp(ast::ExprListComp { elt, .. })
                    | Expr::SetComp(ast::ExprSetComp { elt, .. })
                    | Expr::Generator(ast::ExprGenerator { elt, .. }) => self.visit_expr(elt),
                    Expr::DictComp(ast::ExprDictComp { key, value, .. }) => {
                        if let Some(key) = key {
                            self.visit_expr(key);
                        }
                        self.visit_expr(value);
                    }
                    _ => {}
                }
                self.path.pop();
            }
            _ => walk_expr(self, expr),
        }
    }
}

// `"sep".join(iterable)`.
fn is_join(call: &ast::ExprCall) -> bool {
    call.func.as_attribute_expr().is_some_and(|attribute| {
        attribute.attr.as_str() == "join"
            && matches!(attribute.value.as_ref(), Expr::StringLiteral(_) | Expr::BytesLiteral(_))
    })
}
