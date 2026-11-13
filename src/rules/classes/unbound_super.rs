use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

// Comprehensions run inline in their function from Python 3.12 (PEP 709), so `super()` in one finds the method's
// arguments there.
const INLINED_COMPREHENSIONS: PythonVersion = PythonVersion::PY312;

// Zero-argument `super()` reads the `__class__` cell and the first argument of the function it runs in. A generator
// expression's first argument is the iterator it loops over, a lambda or nested function without parameters has none,
// and a function outside a class has no `__class__` cell, so each call fails when it runs.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return;
    };
    let checker = walker.checker;
    if !arguments.is_empty() || !checker.semantic().is_builtin(func, "super") {
        return;
    }
    let target = checker.target_version();
    // The innermost function-like node around the call decides what `super()` binds to.
    for outer in walker.expressions().iter().rev().skip(1) {
        // The first iterable of a comprehension is evaluated in the enclosing scope, before the comprehension runs.
        if let Some(first) = first_iterable(outer)
            && first.range().contains_range(expr.range())
        {
            continue;
        }
        let reason = match outer {
            Expr::Generator(_) => "a generator expression passes `super()` its iterator instead of the instance",
            Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) if target < INLINED_COMPREHENSIONS => {
                "before Python 3.12 a comprehension passes `super()` its iterator instead of the instance"
            }
            Expr::Lambda(lambda) if lambda.parameters.as_ref().is_none_or(|parameters| !has_positional(parameters)) => {
                "a lambda without parameters has no first argument for `super()` to bind"
            }
            Expr::Lambda(_) => return,
            _ => continue,
        };
        report(walker, expr, reason);
        return;
    }
    let scopes = walker.scopes();
    let reason = match scopes {
        _ if !scopes.iter().any(|scope| matches!(scope, Scope::Class(_))) => {
            "a function outside a class has no `__class__` cell for `super()` to read"
        }
        [.., Scope::Function(_), Scope::Function(inner)] if !has_positional(&inner.parameters) => {
            "a nested function without parameters has no first argument for `super()` to bind"
        }
        [.., Scope::Class(_)] => "a class body has no instance for `super()` to bind",
        _ => return,
    };
    report(walker, expr, reason);
}

fn first_iterable(expr: &Expr) -> Option<&Expr> {
    let first = match expr {
        Expr::Generator(ast::ExprGenerator { generators, .. })
        | Expr::ListComp(ast::ExprListComp { generators, .. })
        | Expr::SetComp(ast::ExprSetComp { generators, .. }) => generators.first(),
        Expr::DictComp(ast::ExprDictComp { generators, .. }) => generators.first(),
        _ => return None,
    };
    first.map(|generator| &generator.iter)
}

fn has_positional(parameters: &ast::Parameters) -> bool {
    !parameters.posonlyargs.is_empty() || !parameters.args.is_empty() || parameters.vararg.is_some()
}

fn report(walker: &Walker, expr: &Expr, reason: &str) {
    walker.checker.report(
        Rule::UnboundSuper,
        expr.range(),
        format!(
            "`super()` without arguments fails here: {reason}; pass the class and instance, or call it in the method"
        ),
    );
}
