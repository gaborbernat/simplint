use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Property,
    CachedProperty,
    ClassMethod,
    StaticMethod,
}

impl Kind {
    fn of(checker: &Checker, decorator: &Expr) -> Option<Self> {
        let semantic = checker.semantic();
        if semantic.is_builtin(decorator, "property") {
            Some(Self::Property)
        } else if semantic.resolves_to(decorator, &["functools", "cached_property"]) {
            Some(Self::CachedProperty)
        } else if semantic.is_builtin(decorator, "classmethod") {
            Some(Self::ClassMethod)
        } else if semantic.is_builtin(decorator, "staticmethod") {
            Some(Self::StaticMethod)
        } else {
            None
        }
    }

    fn name(self) -> &'static str {
        match self {
            Self::Property => "property",
            Self::CachedProperty => "cached_property",
            Self::ClassMethod => "classmethod",
            Self::StaticMethod => "staticmethod",
        }
    }
}

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::FunctionDef(function) = stmt else {
        return;
    };
    // Outside a class a descriptor is often built on purpose and set on a class later, so only class bodies count.
    if !matches!(walker.scopes().last(), Some(Scope::Class(_))) {
        return;
    }
    let checker = walker.checker;
    let kinds: Vec<(Option<Kind>, &ast::Decorator)> =
        function.decorator_list.iter().map(|decorator| (Kind::of(checker, &decorator.expression), decorator)).collect();
    let name = function.name.as_str();
    // Decorators apply bottom up, so each pair is the outer decorator above the one it wraps.
    for pair in kinds.windows(2) {
        let [(Some(outer), decorator), (Some(inner), _)] = pair else {
            continue;
        };
        let message = match (outer, inner) {
            // Python 3.11 deprecated `classmethod` wrapping another descriptor, and Python 3.13 removed it.
            (Kind::ClassMethod, Kind::Property | Kind::CachedProperty)
                if checker.target_version() >= PythonVersion::PY311 =>
            {
                format!(
                    "`@classmethod` over `@{}` stopped returning the value in Python 3.13, so `{name}` reads as a \
                     bound method; use a class-level attribute or a metaclass property",
                    inner.name()
                )
            }
            (Kind::Property | Kind::CachedProperty, Kind::StaticMethod | Kind::ClassMethod) => format!(
                "`@{}` calls `{name}` with the instance, which `@{}` does not accept, so reading it raises \
                 `TypeError`; drop `@{}`",
                outer.name(),
                inner.name(),
                inner.name()
            ),
            _ => continue,
        };
        checker.report(Rule::BrokenDecoratorStack, decorator.range(), message);
    }
}
