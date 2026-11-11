use ruff_diagnostics::Applicability;
use ruff_python_ast::Expr;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::modernize::report_redundant_keyword;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(keyword) = call.arguments.find_keyword("encoding").filter(|keyword| is_utf_8(&keyword.value)) else {
        return;
    };
    let checker = walker.checker;
    if !is_text_opener(checker, &call.func) {
        return;
    }
    report_redundant_keyword(
        checker,
        call,
        keyword,
        Rule::Utf8EncodingDefault,
        Applicability::Unsafe,
        "UTF-8 is the default text encoding from Python 3.15, so `encoding=\"utf-8\"` can go",
        "Drop `encoding=\"utf-8\"`",
    );
}

fn is_utf_8(value: &Expr) -> bool {
    let Expr::StringLiteral(literal) = value else {
        return false;
    };
    let normalized = literal.value.to_str().to_ascii_lowercase().replace(['-', ' '], "_");
    // The spellings Python's codec lookup maps to `utf-8`, once case, `-` and spaces are normalized.
    ["utf_8", "utf8", "u8", "utf", "utf8_ucs2", "utf8_ucs4", "cp65001"].contains(&normalized.as_str())
}

fn is_text_opener(checker: &Checker, func: &Expr) -> bool {
    let semantic = checker.semantic();
    if semantic.is_builtin(func, "open")
        || semantic.resolves_to(func, &["io", "open"])
        || semantic.resolves_to(func, &["io", "TextIOWrapper"])
    {
        return true;
    }
    // A module function such as `tarfile.open` has its own `encoding` meaning, so only unresolved receivers count.
    let Expr::Attribute(attribute) = func else {
        return false;
    };
    // A method's receiver is not typed, so a same-named method of an unrelated class is reported too.
    ["open", "read_text", "write_text"].contains(&attribute.attr.as_str()) && semantic.qualified_name(func).is_none()
}
