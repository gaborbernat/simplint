use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::exception_classes::{ClassKey, handled_types, lineage};

// One class an `except` clause names, with the classes it derives from.
struct Caught<'a> {
    expr: &'a Expr,
    lineage: Vec<ClassKey>,
}

pub fn statement<'a>(walker: &mut Walker<'_, 'a>, stmt: &'a Stmt) {
    let Stmt::Try(ast::StmtTry { handlers, .. }) = stmt else {
        return;
    };
    let checker = walker.checker;
    let mut earlier: Vec<(Caught, usize)> = Vec::new();
    for handler in handlers {
        let ast::ExceptHandler::ExceptHandler(handler) = handler;
        let (Some(type_), Some(elements)) = (handler.type_.as_deref(), handled_types(handler)) else {
            continue;
        };
        let line = checker.line_number(handler.start()).get();
        let caught: Vec<Caught> =
            elements.iter().filter_map(|expr| Some(Caught { expr, lineage: lineage(checker, expr)? })).collect();
        let shadowing: Vec<Option<&(Caught, usize)>> = caught
            .iter()
            .map(|entry| earlier.iter().find(|(broader, _)| is_strict_base(broader, entry.lineage[0], &entry.lineage)))
            .collect();
        if elements.len() > 1 && caught.len() == elements.len() && shadowing.iter().all(Option::is_some) {
            let mut lines: Vec<usize> = shadowing.iter().flatten().map(|(_, line)| *line).collect();
            lines.sort_unstable();
            lines.dedup();
            let handlers = match lines.as_slice() {
                [line] => format!("the handler on line {line} catches"),
                many => format!(
                    "the handlers on lines {} catch",
                    many.iter().map(ToString::to_string).collect::<Vec<_>>().join(", ")
                ),
            };
            checker.report(
                Rule::ShadowedExceptHandler,
                type_.range(),
                format!(
                    "{handlers} a base of every class `{}` names first, so this handler never runs",
                    checker.text(type_.range())
                ),
            );
            earlier.extend(caught.into_iter().map(|entry| (entry, line)));
            continue;
        }
        for (index, entry) in caught.iter().enumerate() {
            let name = checker.text(entry.expr.range());
            let message = if let Some((broader, line)) = shadowing[index] {
                let consequence =
                    if elements.len() == 1 { "this handler never runs" } else { "this entry never matches" };
                format!(
                    "`{name}` derives from `{}`, which the handler on line {line} catches first, so {consequence}",
                    checker.text(broader.expr.range())
                )
            } else if let Some(broader) = caught
                .iter()
                .enumerate()
                .find(|(other, broader)| *other != index && is_strict_base(broader, entry.lineage[0], &entry.lineage))
                .map(|(_, broader)| broader)
            {
                // A class named twice is Ruff's duplicate handler report, so only a strict base counts.
                format!(
                    "`{name}` derives from `{}`, which this clause also names, so it adds nothing",
                    checker.text(broader.expr.range())
                )
            } else {
                continue;
            };
            checker.report_fix(
                Rule::ShadowedExceptHandler,
                entry.expr.range(),
                message,
                removal(checker, type_, &elements, entry.expr),
                format!("Remove `{name}`"),
            );
        }
        earlier.extend(caught.into_iter().map(|entry| (entry, line)));
    }
}

fn is_strict_base(broader: &Caught, own: ClassKey, lineage: &[ClassKey]) -> bool {
    let base = broader.lineage[0];
    base != own && lineage.contains(&base)
}

// Dropping an entry from a tuple keeps what the clause catches, since its base stays in front of it. A lone class
// leaves the clause empty, and a comment in the tuple would be lost, so neither gets a fix.
fn removal(checker: &Checker, type_: &Expr, elements: &[&Expr], removed: &Expr) -> Option<Fix> {
    if elements.len() < 2 || checker.has_comment_in(type_.range()) {
        return None;
    }
    let kept: Vec<&str> =
        elements.iter().filter(|expr| expr.range() != removed.range()).map(|expr| checker.text(expr.range())).collect();
    let content = match kept.as_slice() {
        [single] => (*single).to_owned(),
        many => format!("({})", many.join(", ")),
    };
    Some(Fix::safe_edit(Edit::range_replacement(content, type_.range())))
}
