use std::collections::BTreeSet;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::attribute_call;

// A module loaded by a computed name, as `importlib.import_module(f"pkg.{name}")` or a test fixture typed `ModuleType`,
// hands over whichever module that name picks, so each attribute read off it may be a name of any module. `modules` are
// the names bound to module objects, which the scan of the file collects.
pub(super) fn module_reads(body: &[Stmt], modules: &FxHashSet<String>) -> Vec<String> {
    if modules.is_empty() {
        return Vec::new();
    }
    let mut reads = ModuleReads { modules, reads: BTreeSet::new() };
    reads.visit_body(body);
    reads.reads.into_iter().collect()
}

// A module load, or a name already bound to a module, as is or through `typing.cast`.
pub(super) fn is_module_load(expr: &Expr) -> bool {
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

pub(super) fn is_module_type(annotation: &Expr) -> bool {
    UnqualifiedName::from_expr(annotation)
        .is_some_and(|name| matches!(name.segments(), ["ModuleType"] | ["types", "ModuleType"]))
}

struct ModuleReads<'m> {
    modules: &'m FxHashSet<String>,
    reads: BTreeSet<String>,
}

impl<'a> Visitor<'a> for ModuleReads<'_> {
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
