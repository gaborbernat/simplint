use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::syntax::{decorator_callee, yields};

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(function) = stmt else {
        return;
    };
    let checker = walker.checker;
    let semantic = checker.semantic();
    // Few functions are cached, so the decorators are read before the body is walked for a `yield`.
    let caches: Vec<(&ast::Decorator, &str)> = function
        .decorator_list
        .iter()
        .filter_map(|decorator| {
            let callee = decorator_callee(decorator);
            ["cache", "lru_cache", "cached_property"]
                .into_iter()
                .find(|name| semantic.resolves_to(callee, &["functools", name]))
                .map(|name| (decorator, name))
        })
        .collect();
    if caches.is_empty() || !yields(&function.body) {
        return;
    }
    let instance_method = matches!(walker.scopes().last(), Some(Scope::Class(_)))
        && !semantic.decorated_by(&function.decorator_list, &[&["", "staticmethod"], &["", "classmethod"]]);
    for (decorator, name) in caches {
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
