use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Attribute(attribute) = expr else {
        return;
    };
    // `re.PatternError` exists from Python 3.13, and `re.PatternError is re.error` there. An assignment or `del` patches
    // the module, where a rename would patch the other name. A name bound to another module as well may be that one.
    if !attribute.ctx.is_load()
        || !checker.semantic().resolves_to(expr, &["re", "error"])
        || !checker.semantic().is_only_import_of(&attribute.value, &["re"])
    {
        return;
    }
    let fix = Fix::safe_edit(Edit::range_replacement("PatternError".to_owned(), attribute.attr.range()));
    checker.report_fix(
        Rule::ReErrorAlias,
        expr.range(),
        "`re.error` is an alias of `re.PatternError` from Python 3.13",
        Some(fix),
        "Use `re.PatternError`",
    );
}
