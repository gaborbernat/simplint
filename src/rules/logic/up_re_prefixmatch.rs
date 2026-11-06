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
    // `re.prefixmatch` is the same function as `re.match` from Python 3.15, so the rename keeps the behavior. A name
    // imported from `re` is not reported, since renaming it would also rename its uses. An assignment or `del` patches
    // the module, where a rename would patch the other name.
    if !attribute.ctx.is_load() || !checker.semantic().resolves_to(expr, &["re", "match"]) {
        return;
    }
    let fix = Fix::safe_edit(Edit::range_replacement("prefixmatch".to_owned(), attribute.attr.range()));
    checker.report_fix(
        Rule::RePrefixMatch,
        expr.range(),
        "`re.match` is soft deprecated from Python 3.15; `re.prefixmatch` names the same function",
        Some(fix),
        "Use `re.prefixmatch`",
    );
}
