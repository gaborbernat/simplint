use std::collections::BTreeSet;

use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::attribute_call;

// A computed module name may pick any module, so an attribute read off it may name any module's attribute.
pub(super) fn module_reads(body: &[Stmt], modules: &FxHashSet<String>) -> Vec<String> {
    if modules.is_empty() {
        return Vec::new();
    }
    let mut reads = ModuleReads { modules, reads: BTreeSet::new() };
    reads.visit_body(body);
    reads.reads.into_iter().collect()
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

// The module values read by a computed attribute name, as `getattr(module, name)`, or through their `__all__`, which may
// reach any of their names: `qualify` names the module an expression holds.
pub(super) fn dynamic_modules(body: &[Stmt], qualify: impl Fn(&Expr) -> Option<String>) -> Vec<String> {
    let mut found = DynamicModules { targets: Vec::new() };
    found.visit_body(body);
    found.targets.into_iter().filter_map(qualify).collect::<BTreeSet<String>>().into_iter().collect()
}

struct DynamicModules<'a> {
    targets: Vec<&'a Expr>,
}

impl<'a> Visitor<'a> for DynamicModules<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "__all__" => {
                self.targets.push(value);
            }
            _ => {
                if let Some((module, attribute)) = attribute_call(expr, &["getattr", "hasattr"])
                    && !attribute.is_string_literal_expr()
                {
                    self.targets.push(module);
                }
            }
        }
        visitor::walk_expr(self, expr);
    }
}

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

#[cfg(test)]
mod tests {
    use ruff_python_parser::parse_expression;
    use test_case::test_case;

    use super::is_module_type;

    #[test_case("ModuleType", true ; "imported_name")]
    #[test_case("types.ModuleType", true ; "module_attribute")]
    #[test_case("other.ModuleType", false ; "other_module")]
    #[test_case("list[ModuleType]", false ; "subscript")]
    fn is_module_type_reads_the_annotation(annotation: &str, expected: bool) {
        let parsed = parse_expression(annotation).expect("parses");
        assert_eq!(is_module_type(parsed.expr()), expected);
    }
}
