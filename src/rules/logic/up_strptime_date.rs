use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::from_imports;

// Since Python 3.14 `date` and `time` have their own `strptime`, which parse one part without building a datetime.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(outer) = expr else {
        return;
    };
    let Some(part) = outer.func.as_attribute_expr() else {
        return;
    };
    let member = part.attr.as_str();
    if !matches!(member, "date" | "time") || !outer.arguments.is_empty() {
        return;
    }
    let Expr::Call(parse) = part.value.as_ref() else {
        return;
    };
    let checker = walker.checker;
    if !checker.semantic().resolves_to(&parse.func, &["datetime", "datetime", "strptime"]) {
        return;
    }
    let Some(class) = parse.func.as_attribute_expr().map(|attribute| attribute.value.as_ref()) else {
        return;
    };
    let format = parse
        .arguments
        .find_argument_value("format", 1)
        .and_then(Expr::as_string_literal_expr)
        .map(|format| format.value.to_str());
    // `time.strptime` keeps the zone that `.time()` drops and does not check the date that `datetime.strptime` checks,
    // so only a format of time-of-day directives gives the same value. `date.strptime` and `time.strptime` accept the
    // second 60 that `datetime.strptime` rejects, so a seconds directive changes the value of `.date()` too.
    let safe = match member {
        "date" => format.is_some_and(|format| !has_seconds(format)),
        _ => format.is_some_and(time_of_day),
    };
    // Starts after the call's closing parenthesis, so a parenthesized call keeps its parentheses.
    let suffix = TextRange::new(checker.parenthesized_range(part.value.as_ref(), expr.into()).end(), expr.end());
    let fix = class_replacement(checker, class, member)
        .filter(|_| !checker.has_comment_in(class.range()) && !checker.has_comment_in(suffix))
        .map(|(replacement, import)| {
            let class_edit = Edit::range_replacement(replacement, class.range());
            let mut rest = vec![Edit::range_deletion(suffix)];
            rest.extend(import);
            Fix::applicable_edits(class_edit, rest, if safe { Applicability::Safe } else { Applicability::Unsafe })
        });
    checker.report_fix(
        Rule::StrptimeDate,
        expr.range(),
        format!("Parse the {member} with `{member}.strptime`, not `datetime.strptime(...).{member}()`"),
        fix,
        format!("Call `{member}.strptime`"),
    );
}

fn has_seconds(format: &str) -> bool {
    directives(format)
        .is_none_or(|found| found.iter().any(|directive| matches!(directive, 'S' | 'X' | 'c' | 'T' | 'r')))
}

fn directives(format: &str) -> Option<Vec<char>> {
    let mut chars = format.chars();
    let mut found = Vec::new();
    while let Some(character) = chars.next() {
        if character == '%' {
            found.push(chars.next()?);
        }
    }
    Some(found)
}

fn time_of_day(format: &str) -> bool {
    directives(format)
        .is_some_and(|found| found.iter().all(|directive| matches!(directive, 'H' | 'I' | 'M' | 'f' | 'p' | '%')))
}

// A class imported by name gets `date` or `time` added to its import line, unless the file already spells `date` or
// `time` outside an attribute.
fn class_replacement(checker: &Checker, class: &Expr, member: &str) -> Option<(String, Option<Edit>)> {
    match class {
        Expr::Name(name) => {
            if checker.state().unqualified_name_count(checker, member) > 0 {
                return None;
            }
            let alias = class_alias(checker.body(), name.id.as_str())?;
            Some((member.to_owned(), Some(Edit::insertion(format!("{member}, "), alias.start()))))
        }
        _ => Some((format!("{}.{member}", checker.text(class.as_attribute_expr()?.value.as_ref())), None)),
    }
}

fn class_alias<'b>(body: &'b [Stmt], local: &str) -> Option<&'b ast::Alias> {
    from_imports(body, "datetime")
        .flat_map(|import| &import.names)
        .find(|alias| alias.name.as_str() == "datetime" && alias.asname.as_ref().unwrap_or(&alias.name) == local)
}
