use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::scoped_block::identifier_uses;

// Since Python 3.14 `date` and `time` have their own `strptime`, which parse one part without building a datetime.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(outer) = expr else {
        return;
    };
    let Some(part) = outer.func.as_attribute_expr() else {
        return;
    };
    let member = part.attr.as_str();
    if !matches!(member, "date" | "time") || !outer.arguments.args.is_empty() || !outer.arguments.keywords.is_empty() {
        return;
    }
    let Expr::Call(parse) = part.value.as_ref() else {
        return;
    };
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
    // Drops `.date()` or `.time()`, keeping the parentheses around the call when it has them.
    let unwrap = TextRange::new(checker.parenthesized_range(part.value.as_ref(), expr.into()).end(), expr.end());
    let fix = class_replacement(checker, class, member)
        .filter(|_| !checker.has_comment_in(class.range()) && !checker.has_comment_in(unwrap))
        .map(|(replacement, import)| {
            let class_edit = Edit::range_replacement(replacement, class.range());
            let mut rest = vec![Edit::range_deletion(unwrap)];
            rest.extend(import);
            if safe { Fix::safe_edits(class_edit, rest) } else { Fix::unsafe_edits(class_edit, rest) }
        });
    checker.report_fix(
        Rule::StrptimeDate,
        expr.range(),
        format!("Parse the {member} with `{member}.strptime`, not `datetime.strptime(...).{member}()`"),
        fix,
        format!("Call `{member}.strptime`"),
    );
}

// The text the class expression becomes, and the import it needs. `datetime.datetime` takes the module's `date` or
// `time`. A class imported by name gets its own `date` or `time` from the import that binds it, which the file must
// not already spell.
fn class_replacement(checker: &Checker, class: &Expr, member: &str) -> Option<(String, Option<Edit>)> {
    match class {
        Expr::Name(name) => {
            if identifier_uses(checker, TextRange::up_to(checker.source.text_len()), member) > 0 {
                return None;
            }
            let alias = class_alias(checker.body(), name.id.as_str())?;
            Some((member.to_owned(), Some(Edit::insertion(format!("{member}, "), alias.start()))))
        }
        _ => Some((format!("{}.{member}", checker.text(class.as_attribute_expr()?.value.as_ref())), None)),
    }
}

// The alias that imports `datetime.datetime` under the name `local` at module level.
fn class_alias<'b>(body: &'b [Stmt], local: &str) -> Option<&'b ast::Alias> {
    body.iter()
        .filter_map(|stmt| match stmt {
            Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. })
                if module.as_str() == "datetime" =>
            {
                Some(names)
            }
            _ => None,
        })
        .flatten()
        .find(|alias| alias.name.as_str() == "datetime" && alias.asname.as_ref().unwrap_or(&alias.name) == local)
}

// The directives of a format, where `%%` is a literal percent sign; `None` when the format ends in a `%`.
fn directives(format: &str) -> Option<Vec<char>> {
    let mut chars = format.chars();
    let mut found = Vec::new();
    while let Some(c) = chars.next() {
        if c == '%' {
            found.push(chars.next()?);
        }
    }
    Some(found)
}

// Whether a format has only the directives of the time of day, `%H`, `%I`, `%M`, `%f` and `%p`, and `%%`.
fn time_of_day(format: &str) -> bool {
    directives(format).is_some_and(|found| found.iter().all(|d| matches!(d, 'H' | 'I' | 'M' | 'f' | 'p' | '%')))
}

// Whether a format has a seconds directive, or a composite that includes seconds, or is malformed.
fn has_seconds(format: &str) -> bool {
    directives(format).is_none_or(|found| found.iter().any(|d| matches!(d, 'S' | 'X' | 'c' | 'T' | 'r')))
}
