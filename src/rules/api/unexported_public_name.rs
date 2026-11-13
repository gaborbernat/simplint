use ruff_python_ast::helpers::any_over_body;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::api::definitions;
use crate::rules::frameworks::{is_ply_name, uses_ply};
use crate::rules::rename::{kept_message, rename_fix};
use crate::syntax::{Exports, Visibility};

pub fn unexported_public_name(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let Exports::Listed(exported) = checker.dunder_all() else {
        return;
    };
    // A module that finds its own functions by a computed name, as `globals()["do_" + name]`, reaches the names it does
    // not list, so they are not leftovers to hide.
    if any_over_body(checker.body(), &|expr: &Expr| looks_itself_up(checker, expr)) {
        return;
    }
    // Scans the module's statements once, not once per definition.
    let ply = uses_ply(checker);
    for definition in definitions(checker.body().iter()).filter(|definition| !definition.decorated) {
        let identifier = definition.name;
        let name = identifier.as_str();
        if Visibility::of(name) != Visibility::Public
            || is_discovered_by_name(name)
            || ply && is_ply_name(name)
            || exported.contains(name)
            || checker.project().is_used_elsewhere(name)
        {
            continue;
        }
        let private = format!("_{name}");
        // The names of a public module are the API its users import, so a rename breaks them: the author lists the name
        // or renames it by hand.
        if checker.project().role().public {
            checker.report(Rule::UnexportedPublicName, identifier.range(), kept_message(name, &private, false));
            continue;
        }
        // Code that reads the name of a class or function, as `type(self).__name__` or `helper.__name__`, sees the new
        // name once renamed.
        let class = matches!(definition.stmt, Stmt::ClassDef(_));
        let reflected = checker.project().reads_name_of(name) || class && checker.project().reads_class_names();
        checker.report_fix(
            Rule::UnexportedPublicName,
            identifier.range(),
            format!("`{name}` is not in `__all__`; name it `{private}`"),
            (!reflected).then(|| rename_fix(checker, name, &private)).flatten(),
            format!("Rename `{name}` to `{private}`"),
        );
    }
}

fn is_discovered_by_name(name: &str) -> bool {
    name.starts_with("pytest_") || matches!(name, "load_tests" | "setup_module" | "teardown_module" | "setup")
}

// `globals()[key]`, `vars()[key]`, `globals().get(key)` or `getattr(sys.modules[__name__], key)` with a key that is not
// a string literal.
fn looks_itself_up(checker: &Checker, expr: &Expr) -> bool {
    let semantic = checker.semantic();
    let is_namespace = |expr: &Expr| {
        expr.as_call_expr().is_some_and(|call| {
            call.arguments.is_empty()
                && (semantic.is_builtin(&call.func, "globals") || semantic.is_builtin(&call.func, "vars"))
        })
    };
    let computed = |key: &Expr| !key.is_string_literal_expr();
    match expr {
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => is_namespace(value) && computed(slice),
        Expr::Call(ast::ExprCall { func, arguments, .. }) => match func.as_ref() {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "get" => {
                is_namespace(value) && arguments.args.first().is_some_and(computed)
            }
            func if semantic.is_builtin(func, "getattr") => {
                arguments.args.first().is_some_and(|target| is_own_module(checker, target))
                    && arguments.args.get(1).is_some_and(computed)
            }
            _ => false,
        },
        _ => false,
    }
}

// `sys.modules[__name__]`.
fn is_own_module(checker: &Checker, expr: &Expr) -> bool {
    expr.as_subscript_expr().is_some_and(|subscript| {
        checker.semantic().resolves_to(&subscript.value, &["sys", "modules"])
            && subscript.slice.as_name_expr().is_some_and(|name| name.id.as_str() == "__name__")
    })
}
