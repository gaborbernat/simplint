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

// The module values read by a computed attribute name, as `getattr(module, name)`, or through their `__all__`, which
// may reach any of their names: `qualify` names the module an expression holds. Also whether an `__all__` is read off a
// module imported by a computed name, as `__import__(name).__all__` or `importlib.import_module(name).__all__` do.
pub(super) fn dynamic_modules(body: &[Stmt], qualify: impl Fn(&Expr) -> Option<String>) -> DynamicReads {
    let mut found = DynamicModules::default();
    found.visit_body(body);
    let unknown_all = found.all_reads.iter().any(|value| match value {
        Expr::Name(name) => found.loaded.contains(name.id.as_str()),
        other => is_computed_load(other),
    });
    let modules = found.targets.into_iter().chain(found.all_reads).filter_map(qualify).collect::<BTreeSet<String>>();
    DynamicReads { modules: modules.into_iter().collect(), unknown_all }
}

#[derive(Clone, Default, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(in crate::project) struct DynamicReads {
    // Modules read by a computed attribute name or through their `__all__`, whose functions any call may reach.
    pub(in crate::project) modules: Vec<String>,
    // Whether an `__all__` is read off a module imported by a computed name, so any module's list may be read.
    pub(in crate::project) unknown_all: bool,
}

#[derive(Default)]
struct DynamicModules<'a> {
    targets: Vec<&'a Expr>,
    all_reads: Vec<&'a Expr>,
    // The names bound to a module imported by a computed name.
    loaded: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for DynamicModules<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt
            && let [Expr::Name(target)] = targets.as_slice()
            && is_computed_load(value)
        {
            self.loaded.insert(target.id.as_str());
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "__all__" => {
                self.all_reads.push(value);
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

// A module the code picks by a computed name: an import call, a `getattr`, or a `sys.modules` entry other than a
// literal name or the module's own.
fn is_computed_load(expr: &Expr) -> bool {
    match expr {
        Expr::Subscript(ast::ExprSubscript { slice, .. }) => {
            is_module_load(expr)
                && !slice.is_string_literal_expr()
                && slice.as_name_expr().is_none_or(|name| name.id.as_str() != "__name__")
        }
        Expr::Call(_) => is_module_load(expr) || attribute_call(expr, &["getattr"]).is_some(),
        _ => false,
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
    use ruff_python_ast::Expr;
    use ruff_python_parser::{parse_expression, parse_module};
    use test_case::test_case;

    use super::{dynamic_modules, is_module_type};

    #[test_case("mod = __import__(name)\nmod.__all__\n", true ; "bound_import")]
    #[test_case("importlib.import_module(name).__all__\n", true ; "import_call")]
    #[test_case("getattr(pkg, name).__all__\n", true ; "getattr_call")]
    #[test_case("sys.modules[name].__all__\n", true ; "computed_sys_modules")]
    #[test_case("sys.modules['json'].__all__\n", false ; "literal_sys_modules")]
    #[test_case("sys.modules[__name__].__all__\n", false ; "own_module")]
    #[test_case("items[name].__all__\n", false ; "other_subscript")]
    #[test_case("mod = load(name)\nmod.__all__\n", false ; "other_binding")]
    #[test_case("mod, other = __import__(name), 1\nmod.__all__\n", false ; "unpacked_binding")]
    #[test_case("pkg.mod.__all__\n", false ; "dotted_module")]
    fn dynamic_modules_finds_a_computed_all_read(source: &str, expected: bool) {
        let parsed = parse_module(source).expect("parses");
        assert_eq!(dynamic_modules(&parsed.syntax().body, |_: &Expr| None).unknown_all, expected);
    }

    #[test]
    fn dynamic_modules_names_modules_read_by_a_computed_name() {
        let parsed =
            parse_module("getattr(a, name)\ngetattr(b, 'literal')\nhasattr(c, name)\nd.__all__\n").expect("parses");
        let named = |expr: &Expr| expr.as_name_expr().map(|name| format!("pkg.{}", name.id));
        assert_eq!(dynamic_modules(&parsed.syntax().body, named).modules, ["pkg.a", "pkg.c", "pkg.d"]);
    }

    #[test_case("ModuleType", true ; "imported_name")]
    #[test_case("types.ModuleType", true ; "module_attribute")]
    #[test_case("other.ModuleType", false ; "other_module")]
    #[test_case("list[ModuleType]", false ; "subscript")]
    fn is_module_type_reads_the_annotation(annotation: &str, expected: bool) {
        let parsed = parse_expression(annotation).expect("parses");
        assert_eq!(is_module_type(parsed.expr()), expected);
    }
}
