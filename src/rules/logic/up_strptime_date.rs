use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::scoped_block::identifier_uses;

// Since Python 3.14 `date` and `time` have their own `strptime`, which parse one part without building a datetime.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    if checker.settings.from_ruff.target_version < PythonVersion::PY314 {
        return;
    }
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
    // `time.strptime` keeps the zone that `.time()` drops, so a format with `%z` may give another value.
    let plain_format = parse
        .arguments
        .find_argument_value("format", 1)
        .and_then(Expr::as_string_literal_expr)
        .is_some_and(|format| !format.value.to_str().contains("%z"));
    let fix = class_replacement(checker, class, member).map(|(replacement, import)| {
        let class_edit = Edit::range_replacement(replacement, class.range());
        // Drops `.date()` or `.time()`, keeping the parentheses around the call when it has them.
        let unwrap = Edit::range_deletion(TextRange::new(
            checker.parenthesized_range(part.value.as_ref(), expr.into()).end(),
            expr.end(),
        ));
        let mut rest = vec![unwrap];
        rest.extend(import);
        if member == "date" || plain_format {
            Fix::safe_edits(class_edit, rest)
        } else {
            Fix::unsafe_edits(class_edit, rest)
        }
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
