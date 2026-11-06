use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::Expr;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::keyword_removal;

// Text reads and writes that take an `encoding` argument, and the `pathlib` methods named like them. A method's
// receiver is not typed, so an unrelated class with the same method name is reported too.
const PATH_METHODS: [&str; 3] = ["open", "read_text", "write_text"];

// The spellings of UTF-8 Python's codec lookup maps to `utf-8`, after its normalization of case, `-` and spaces.
const UTF_8_NAMES: [&str; 7] = ["utf_8", "utf8", "u8", "utf", "utf8_ucs2", "utf8_ucs4", "cp65001"];

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(keyword) = call.arguments.find_keyword("encoding").filter(|keyword| is_utf_8(&keyword.value)) else {
        return;
    };
    if !is_text_opener(checker, &call.func) {
        return;
    }
    let fix = keyword_removal(checker, call, keyword).map(|range| Fix::unsafe_edit(Edit::range_deletion(range)));
    checker.report_fix(
        Rule::Utf8EncodingDefault,
        keyword.range(),
        "UTF-8 is the default text encoding from Python 3.15, so `encoding=\"utf-8\"` can go",
        fix,
        "Drop `encoding=\"utf-8\"`",
    );
}

fn is_utf_8(value: &Expr) -> bool {
    let Expr::StringLiteral(literal) = value else {
        return false;
    };
    let normalized = literal.value.to_str().to_ascii_lowercase().replace(['-', ' '], "_");
    UTF_8_NAMES.contains(&normalized.as_str())
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
    PATH_METHODS.contains(&attribute.attr.as_str()) && semantic.qualified_name(func).is_none()
}
