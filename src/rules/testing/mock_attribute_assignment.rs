use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::MOCK_CLASSES;
use crate::rules::{expr_mentions, for_each_body, is_alone_on_lines, last_segment, single_assignment};

pub fn mock_attribute_assignment(checker: &Checker) {
    for_each_body(checker.body(), &mut |statements, _| {
        for (index, stmt) in statements.iter().enumerate() {
            let Some((target, _, Expr::Call(call))) = single_assignment(stmt) else {
                continue;
            };
            if !last_segment(&call.func).is_some_and(|last| MOCK_CLASSES.contains(&last) || last == "create_autospec") {
                continue;
            }
            let assignments = following_assignments(target.id.as_str(), &statements[index + 1..]);
            if !assignments.is_empty() {
                report_assignments(checker, call, &assignments);
            }
        }
    });
}

struct Assignment<'a> {
    stmt: &'a Stmt,
    attr: &'a str,
    value: &'a Expr,
}

fn following_assignments<'a>(mock: &str, statements: &'a [Stmt]) -> Vec<Assignment<'a>> {
    // These constructor arguments configure the mock itself, so passing one differs from setting the attribute.
    const RESERVED_KEYWORDS: &[&str] =
        &["spec", "spec_set", "wraps", "name", "parent", "unsafe", "instance", "_name", "_parent"];
    let mut assignments: Vec<Assignment> = Vec::new();
    for stmt in statements {
        let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
            break;
        };
        let [Expr::Attribute(ast::ExprAttribute { value: owner, attr, .. })] = targets.as_slice() else {
            break;
        };
        let attr = attr.as_str();
        if owner.as_name_expr().is_none_or(|name| name.id.as_str() != mock)
            || RESERVED_KEYWORDS.contains(&attr)
            || assignments.iter().any(|existing| existing.attr == attr)
            || expr_mentions(value, mock)
        {
            break;
        }
        assignments.push(Assignment { stmt, attr, value });
    }
    assignments
}

fn report_assignments(checker: &Checker, call: &ast::ExprCall, assignments: &[Assignment]) {
    let source = checker.source;
    let is_autospec = last_segment(&call.func) == Some("create_autospec");
    // A `**` splat may already carry any of the attributes, so the merged call could repeat a keyword.
    let fixable = call.arguments.keywords.iter().all(|keyword| keyword.arg.is_some())
        && assignments.iter().all(|assignment| is_alone_on_lines(source, assignment.stmt.range()))
        && assignments
            .iter()
            .all(|assignment| !checker.has_comment_in(source.full_lines_range(assignment.stmt.range())));
    let fix = fixable.then(|| {
        let close = call.arguments.end() - TextSize::from(1);
        let mut keywords = Vec::new();
        let mut edits: Vec<Edit> = assignments
            .iter()
            .map(|assignment| Edit::range_deletion(source.full_lines_range(assignment.stmt.range())))
            .collect();
        for assignment in assignments {
            let text = checker.text(assignment.value.range());
            // As a keyword argument, a bare tuple would split into arguments, and `:=` or `yield` need brackets.
            let bare = match assignment.value {
                Expr::Tuple(tuple) => !tuple.parenthesized,
                Expr::Named(_) | Expr::Yield(_) | Expr::YieldFrom(_) => true,
                _ => false,
            };
            let value = if bare { format!("({text})") } else { text.to_owned() };
            if let Some(existing) = call.arguments.find_keyword(assignment.attr) {
                edits.push(Edit::range_replacement(value, existing.value.range()));
            } else {
                keywords.push(format!("{}={value}", assignment.attr));
            }
        }
        if !keywords.is_empty() {
            let separator = if call.arguments.is_empty() {
                ""
            } else if checker.text(TextRange::new(call.arguments.start(), close)).trim_end().ends_with(',') {
                " "
            } else {
                ", "
            };
            edits.push(Edit::insertion(format!("{separator}{}", keywords.join(", ")), close));
        }
        let first = edits.remove(0);
        // `create_autospec` of a function returns a wrapper that hands other keywords to the inner mock, so only
        // `return_value` and `side_effect` reach the object the test holds.
        if is_autospec
            && !assignments.iter().all(|assignment| matches!(assignment.attr, "return_value" | "side_effect"))
        {
            Fix::unsafe_edits(first, edits)
        } else {
            Fix::safe_edits(first, edits)
        }
    });
    let names: Vec<&str> = assignments.iter().map(|assignment| assignment.attr).collect();
    checker.report_fix(
        Rule::MockAttributeAssignment,
        assignments[0].stmt.range(),
        format!("Pass {} through the mock constructor instead of assigning after creation", names.join(", ")),
        fix,
        format!("Pass {} to the mock constructor", names.join(", ")),
    );
}
