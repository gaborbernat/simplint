use std::collections::BTreeSet;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::{ReturnVisitor, attribute_call};

// A module loaded by a computed name, as `importlib.import_module(f"pkg.{name}")` or a test fixture typed `ModuleType`,
// hands over whichever module that name picks, so each attribute read off it may be a name of any module.
pub(super) fn module_reads(body: &[Stmt]) -> Vec<String> {
    let mut modules = ModuleObjects::default();
    modules.visit_body(body);
    if modules.names.is_empty() {
        return Vec::new();
    }
    let mut reads = ModuleReads { modules: &modules.names, reads: BTreeSet::new() };
    reads.visit_body(body);
    reads.reads.into_iter().collect()
}

// The names the file binds to module objects, wherever it binds them; a name bound to a module anywhere counts
// everywhere, which can only keep more names.
#[derive(Default)]
struct ModuleObjects<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for ModuleObjects<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let (targets, value, annotation) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => (targets.as_slice(), Some(value.as_ref()), None),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value, annotation, .. }) => {
                (std::slice::from_ref(target.as_ref()), value.as_deref(), Some(annotation.as_ref()))
            }
            _ => (&[][..], None, None),
        };
        if value.is_some_and(|value| self.is_module(value)) || annotation.is_some_and(is_module_type) {
            self.names.extend(targets.iter().filter_map(Expr::as_name_expr).map(|name| name.id.as_str()));
        }
        visitor::walk_stmt(self, stmt);
        // A pytest fixture that returns a module hands it to each test under the fixture's name.
        if let Stmt::FunctionDef(function) = stmt {
            let mut returns = ReturnVisitor::default();
            returns.visit_body(&function.body);
            let returns_module =
                returns.returns.iter().filter_map(|stmt| stmt.value.as_deref()).any(|value| self.is_module(value));
            if returns_module {
                self.names.insert(function.name.as_str());
                self.names.extend(
                    function
                        .decorator_list
                        .iter()
                        .filter_map(|decorator| decorator.expression.as_call_expr()?.arguments.find_keyword("name"))
                        .filter_map(|keyword| Some(keyword.value.as_string_literal_expr()?.value.to_str())),
                );
            }
        }
    }

    fn visit_parameter(&mut self, parameter: &'a ast::Parameter) {
        if parameter.annotation.as_deref().is_some_and(is_module_type) {
            self.names.insert(parameter.name.as_str());
        }
        visitor::walk_parameter(self, parameter);
    }
}

impl ModuleObjects<'_> {
    // A module load, or a name already bound to a module, as is or through `typing.cast`.
    fn is_module(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => self.names.contains(name.id.as_str()),
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().last() == Some(&"cast")) =>
            {
                arguments.args.get(1).is_some_and(|value| self.is_module(value))
            }
            _ => is_module_load(expr),
        }
    }
}

fn is_module_load(expr: &Expr) -> bool {
    match expr {
        Expr::Call(ast::ExprCall { func, .. }) => match func.as_ref() {
            Expr::Name(name) => matches!(name.id.as_str(), "import_module" | "__import__"),
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => attr == "import_module",
            _ => false,
        },
        Expr::Subscript(ast::ExprSubscript { value, .. }) => {
            UnqualifiedName::from_expr(value).is_some_and(|name| name.segments() == ["sys", "modules"])
        }
        _ => false,
    }
}

fn is_module_type(annotation: &Expr) -> bool {
    UnqualifiedName::from_expr(annotation)
        .is_some_and(|name| matches!(name.segments(), ["ModuleType"] | ["types", "ModuleType"]))
}

struct ModuleReads<'a, 'm> {
    modules: &'m FxHashSet<&'a str>,
    reads: BTreeSet<String>,
}

impl<'a> Visitor<'a> for ModuleReads<'a, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let read = match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                value.as_name_expr().filter(|name| self.modules.contains(name.id.as_str())).map(|_| attr.as_str())
            }
            // A `hasattr` probe reads the module's name as much as `getattr` does.
            _ => match attribute_call(expr, &["getattr", "hasattr"]) {
                Some((Expr::Name(module), Expr::StringLiteral(attribute)))
                    if self.modules.contains(module.id.as_str()) =>
                {
                    Some(attribute.value.to_str())
                }
                _ => None,
            },
        };
        self.reads.extend(read.map(str::to_owned));
        visitor::walk_expr(self, expr);
    }
}
