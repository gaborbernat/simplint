use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr};

use crate::semantic::Semantic;
use crate::syntax::{MOCK_CLASSES, decorator_callee, last_segment};

pub(crate) fn mock_constructor(semantic: &Semantic, func: &Expr) -> Option<&'static str> {
    let qualified = semantic.qualified_name(func);
    let name = if let Some(ref name) = qualified {
        let (["unittest", "mock", name] | ["mock", name]) = name.segments() else { return None };
        *name
    } else {
        let attribute = func.as_attribute_expr()?;
        let receiver = attribute.value.as_name_expr()?;
        let annotation = semantic.unambiguous_parameter_annotation(receiver)?;
        if !semantic.resolves_to(annotation, &["pytest_mock", "MockerFixture"]) {
            return None;
        }
        attribute.attr.as_str()
    };
    MOCK_CLASSES.iter().copied().find(|&class| class == name)
}

// `create_autospec` takes the spec it builds the mock from, so it is never built without one.
pub(crate) fn is_mock_constructor_except_autospec(func: &Expr) -> bool {
    last_segment(func).is_some_and(|last| MOCK_CLASSES.contains(&last) && last != "create_autospec")
}

// `patch` alone is too common a method name (HTTP clients, `requests.patch`) to match on the name; the call must
// reach it through a mock module or the pytest-mock fixture.
pub(crate) fn is_mock_patch(semantic: &Semantic, func: &Expr) -> bool {
    let resolved = semantic.qualified_name(func);
    resolved.is_some_and(|name| matches!(name.segments(), ["unittest", "mock", "patch"] | ["mock", "patch"]))
        || UnqualifiedName::from_expr(func).is_some_and(|name| matches!(name.segments(), [.., "mocker", "patch"]))
}

pub(crate) fn is_patch_object(semantic: &Semantic, func: &Expr) -> bool {
    last_segment(func) == Some("object")
        && func.as_attribute_expr().is_some_and(|attribute| is_mock_patch(semantic, &attribute.value))
}

pub(crate) fn is_patcher(semantic: &Semantic, func: &Expr) -> bool {
    const PATCHERS: &[&[&str]] = &[
        &["unittest", "mock", "patch"],
        &["unittest", "mock", "patch", "object"],
        &["unittest", "mock", "patch", "dict"],
        &["mock", "patch"],
        &["mock", "patch", "object"],
        &["mock", "patch", "dict"],
    ];
    PATCHERS.iter().any(|path| semantic.resolves_to(func, path))
}

pub(crate) fn is_patch_by_name(func: &Expr) -> bool {
    last_segment(func) == Some("patch")
}

pub(crate) fn is_fixture(decorator: &ast::Decorator) -> bool {
    last_segment(decorator_callee(decorator)) == Some("fixture")
}

pub(crate) fn expects_error(expr: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return false;
    };
    last_segment(func).is_some_and(|name| name == "raises" || name.starts_with("assertRaises"))
}

pub(crate) fn is_assert_raises_family(func: &Expr) -> bool {
    last_segment(func).is_some_and(|name| name.starts_with("assertRaises"))
}

pub(crate) fn is_raises_or_assert_raises(func: &Expr) -> bool {
    last_segment(func).is_some_and(|name| matches!(name, "raises" | "assertRaises"))
}

pub(crate) fn is_pytest_raises(semantic: &Semantic, func: &Expr) -> bool {
    semantic.resolves_to(func, &["pytest", "raises"])
}
