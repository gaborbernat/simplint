use ruff_diagnostics::Fix;
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::modernize::{module_attribute_read, rename_attribute};

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    // `re.PatternError` exists from Python 3.13, and `re.PatternError is re.error` there.
    let Some(attribute) = module_attribute_read(checker, expr, &["re", "error"]) else {
        return;
    };
    checker.report_fix(
        Rule::ReErrorAlias,
        expr.range(),
        "`re.error` is an alias of `re.PatternError` from Python 3.13",
        Some(Fix::safe_edit(rename_attribute(attribute, "PatternError"))),
        "Use `re.PatternError`",
    );
}
