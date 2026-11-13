use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::rules::logic::name_uses::comprehension_generators;
use crate::syntax::{alias_root_binding, target_names};

// The nested functions, lambdas and comprehensions a walk is inside, with the names each binds for itself. A mention
// of such a name inside the scope reads the scope's own variable, never the enclosing one of the same spelling.
#[derive(Default)]
pub(in crate::rules::logic) struct InnerScopes<'a> {
    stack: Vec<InnerScope<'a>>,
}

struct InnerScope<'a> {
    // The source the scope's names cover: a function's body, a lambda's body, or a comprehension without the iterable
    // of its first `for`, which runs in the enclosing scope.
    region: TextRange,
    outside: Option<TextRange>,
    locals: FxHashSet<&'a str>,
}

impl<'a> InnerScopes<'a> {
    // Call before walking the statement; when it returns `true`, call `leave` after.
    pub(in crate::rules::logic) fn enter_stmt(&mut self, stmt: &'a Stmt) -> bool {
        let Stmt::FunctionDef(function) = stmt else {
            return false;
        };
        let mut locals = Locals::default();
        locals.visit_body(&function.body);
        let bound = function
            .parameters
            .iter()
            .map(|parameter| parameter.name().as_str())
            .chain(locals.bound)
            .filter(|name| !locals.declared.contains(name))
            .collect();
        self.stack.push(InnerScope { region: body_region(&function.body), outside: None, locals: bound });
        true
    }

    pub(in crate::rules::logic) fn enter_expr(&mut self, expr: &'a Expr) -> bool {
        let scope = if let Expr::Lambda(lambda) = expr {
            InnerScope {
                region: lambda.body.range(),
                outside: None,
                locals: lambda
                    .parameters
                    .iter()
                    .flat_map(|parameters| parameters.iter())
                    .map(|parameter| parameter.name().as_str())
                    .collect(),
            }
        } else {
            let generators = comprehension_generators(expr);
            let Some(first) = generators.first() else {
                return false;
            };
            InnerScope {
                region: expr.range(),
                outside: Some(first.iter.range()),
                locals: generators.iter().flat_map(|generator| target_names(&generator.target)).collect(),
            }
        };
        self.stack.push(scope);
        true
    }

    pub(in crate::rules::logic) fn leave(&mut self) {
        self.stack.pop();
    }

    // Whether a nested scope the walk is in binds the name at `range` for itself.
    pub(in crate::rules::logic) fn binds(&self, name: &str, range: TextRange) -> bool {
        self.owner(name, range).is_some()
    }

    // The region of the innermost nested scope that binds the name at `range` for itself.
    pub(in crate::rules::logic) fn owner(&self, name: &str, range: TextRange) -> Option<TextRange> {
        self.stack
            .iter()
            .rev()
            .find(|scope| {
                scope.locals.contains(name)
                    && scope.region.contains_range(range)
                    && scope.outside.is_none_or(|outside| !outside.contains_range(range))
            })
            .map(|scope| scope.region)
    }
}

// A function body is never empty.
pub(in crate::rules::logic) fn body_region(body: &[Stmt]) -> TextRange {
    TextRange::new(body[0].start(), body[body.len() - 1].end())
}

// The names a function body binds, outside its nested scopes, and those it declares `global` or `nonlocal`. Bindings
// the walk does not follow, such as an `except` target, leave the name to the enclosing scope, which only keeps more.
#[derive(Default)]
struct Locals<'a> {
    bound: Vec<&'a str>,
    declared: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for Locals<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
                self.bound.push(name.as_str());
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.bound.extend(names.iter().map(alias_root_binding));
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.declared.extend(names.iter().map(ast::Identifier::as_str));
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.bound.push(id.as_str());
            }
            Expr::Lambda(_) => {}
            _ if !comprehension_generators(expr).is_empty() => {}
            _ => visitor::walk_expr(self, expr),
        }
    }
}
