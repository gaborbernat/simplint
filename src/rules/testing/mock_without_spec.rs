use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::testing::recognize::{is_mock_constructor_except_autospec, is_mock_patch, is_patch_object};
use crate::semantic::Semantic;
use crate::syntax::last_segment;

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
            _ if is_mock_constructor_except_autospec(func) => Some(Self::Mock),
            _ if is_mock_patch(semantic, func) => Some(Self::Patch),
            _ if is_patch_object(semantic, func) => Some(Self::PatchObject),
            _ => None,
        }
    }
}
