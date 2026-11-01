use ruff_python_ast::Expr;
use ruff_python_ast::name::UnqualifiedName;
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::last_segment;
use crate::rules::testing::MOCK_CLASSES;
use crate::semantic::Semantic;

pub fn expression(walker: &mut Walker, expr: &Expr) {
    const SPEC_KEYWORDS: &[&str] = &["spec", "spec_set", "wraps", "autospec", "new", "new_callable"];
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(kind) = MockCall::of(walker.checker.semantic(), &call.func) else {
        return;
    };
    let positional_spec = match kind {
        MockCall::Mock => 1,
        MockCall::Patch => 2,
        MockCall::PatchObject => 3,
    };
    let specced = call.arguments.args.len() >= positional_spec
        || call.arguments.keywords.iter().any(|keyword| {
            // `spec=None` and `autospec=False` spell out the default: no spec. `new` is the replacement itself, so any
            // value, `None` and `False` included, means no mock is created.
            keyword.arg.as_ref().is_none_or(|arg| arg == "new")
                || (keyword.arg.as_ref().is_some_and(|arg| SPEC_KEYWORDS.contains(&arg.as_str()))
                    && !matches!(&keyword.value, Expr::NoneLiteral(_))
                    && keyword.value.as_boolean_literal_expr().is_none_or(|literal| literal.value))
        });
    if specced {
        return;
    }
    let message = match kind {
        MockCall::Mock => "Mock without a spec; build it with `create_autospec` or pass `spec=`",
        MockCall::Patch | MockCall::PatchObject => "Patch without a spec; pass `autospec=True`",
    };
    walker.checker.report(Rule::MockWithoutSpec, call.range(), message);
}

enum MockCall {
    Mock,
    Patch,
    PatchObject,
}

impl MockCall {
    fn of(semantic: &Semantic, func: &Expr) -> Option<Self> {
        match last_segment(func)? {
            last if MOCK_CLASSES.contains(&last) => Some(Self::Mock),
            _ if is_patch(semantic, func) => Some(Self::Patch),
            "object" if func.as_attribute_expr().is_some_and(|attribute| is_patch(semantic, &attribute.value)) => {
                Some(Self::PatchObject)
            }
            _ => None,
        }
    }
}

// `patch` alone is too common a method name (HTTP clients, `requests.patch`) to match on the name; the call must
// reach it through a mock module or the pytest-mock fixture.
fn is_patch(semantic: &Semantic, func: &Expr) -> bool {
    semantic.resolves_to(func, &["unittest", "mock", "patch"])
        || semantic.resolves_to(func, &["mock", "patch"])
        || UnqualifiedName::from_expr(func).is_some_and(|name| matches!(name.segments(), [.., "mocker", "patch"]))
}
