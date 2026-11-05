use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::scoped_block::identifier_uses;
use crate::syntax::for_each_body;

// Python 3.15 adds the `sentinel` builtin, whose repr and pickle carry the name the placeholder is bound to.
pub fn up_object_sentinel(checker: &Checker) {
    if identifier_uses(checker, TextRange::up_to(checker.source.text_len()), "sentinel") > 0 {
        return;
    }
    // Only module and class bodies hold the placeholders the rule means; a function's local is not reachable by name.
    for_each_body(checker.body(), &mut |body, owner| {
        if owner.is_some_and(|owner| !matches!(owner, Stmt::ClassDef(_))) {
            return;
        }
        for stmt in body {
            let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
                continue;
            };
            let [Expr::Name(target)] = targets.as_slice() else {
                continue;
            };
            if is_bare_object(checker, value) && !checker.semantic().is_rebound(target.id.as_str()) {
                report(checker, target.id.as_str(), value);
            }
        }
    });
}

fn is_bare_object(checker: &Checker, value: &Expr) -> bool {
    matches!(value, Expr::Call(ast::ExprCall { func, arguments, .. })
        if arguments.args.is_empty()
            && arguments.keywords.is_empty()
            && checker.semantic().is_builtin(func, "object"))
}

fn report(checker: &Checker, name: &str, value: &Expr) {
    // `sentinel` changes what the name reprs as and how it pickles, which a program can observe.
    let fix = Fix::unsafe_edit(Edit::range_replacement(format!("sentinel(\"{name}\")"), value.range()));
    checker.report_fix(
        Rule::ObjectSentinel,
        value.range(),
        format!("`{name}` is a bare `object()`, whose repr is an address; `sentinel(\"{name}\")` names it"),
        Some(fix),
        "Create it with `sentinel`",
    );
}
