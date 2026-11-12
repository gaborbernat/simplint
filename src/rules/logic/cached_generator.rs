use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::yields;

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(function) = stmt else {
        return;
    };
    if !yields(&function.body) {
        return;
    }
    let checker = walker.checker;
    let semantic = checker.semantic();
    let instance_method = matches!(walker.scopes().last(), Some(Scope::Class(_)))
        && !function.decorator_list.iter().any(|decorator| {
            semantic.is_builtin(&decorator.expression, "staticmethod")
                || semantic.is_builtin(&decorator.expression, "classmethod")
        });
    for decorator in &function.decorator_list {
        let callee = match &decorator.expression {
            Expr::Call(ast::ExprCall { func, .. }) => func.as_ref(),
            expression => expression,
        };
        let Some(name) = ["cache", "lru_cache", "cached_property"]
            .into_iter()
            .find(|name| semantic.resolves_to(callee, &["functools", name]))
        else {
            continue;
        };
        // Ruff's cached-instance-method (B019) reports `cache` and `lru_cache` on an instance method.
        if instance_method && name != "cached_property" {
            continue;
        }
        let (article, what) = if function.is_async { ("an", "async generator") } else { ("a", "generator") };
        checker.report(
            Rule::CachedGenerator,
            decorator.range(),
            format!(
                "`functools.{name}` on {article} {what} keeps the {what} object, which the first caller exhausts, so every \
                 later call gets nothing; cache a list of the values instead"
            ),
        );
    }
}
