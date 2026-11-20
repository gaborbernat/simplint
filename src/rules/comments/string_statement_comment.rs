use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::{self as ast, Stmt, StringFlags};
use ruff_python_trivia::textwrap::dedent;
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::{is_tool_directive, owns_docstring};
use crate::semantic::future_import;
use crate::syntax::for_each_body;

pub fn string_statement_comment(checker: &Checker) {
    // Removing an earlier string can change whether Python accepts a `from __future__` import.
    let has_future_import = checker.body().iter().any(|stmt| future_import(stmt).is_some());
    for_each_body(checker.body(), &mut |body, owner| {
        for (index, stmt) in body.iter().enumerate() {
            let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
                continue;
            };
            let Some(literal) = value.as_string_literal_expr() else {
                continue;
            };
            // A tracer or test can use a short string statement as a line marker.
            if !literal.value.iter().all(|part| part.flags.is_triple_quoted())
                || is_docstring(index, owner)
                || is_attribute_docstring(body, index)
            {
                continue;
            }
            let fix = (!has_future_import).then(|| comment_replacement(checker, body, stmt, literal)).flatten();
            checker.report_fix(
                Rule::StringStatementComment,
                stmt.range(),
                "String statement is not a docstring; write it as `#` comments",
                fix.map(Fix::safe_edit),
                "Write the string as `#` comments",
            );
        }
    });
}

fn is_docstring(index: usize, owner: Option<&Stmt>) -> bool {
    index == 0 && owns_docstring(owner)
}

// Source-based documentation tools can read strings following assignments as attribute documentation.
fn is_attribute_docstring(body: &[Stmt], index: usize) -> bool {
    index > 0 && matches!(body[index - 1], Stmt::Assign(_) | Stmt::AnnAssign(_))
}

// Preserve line numbers for the code after the string. Decline the edit if it would hide escapes, create a tool
// directive, erase other source on the same line or leave a body empty.
fn comment_replacement(
    checker: &Checker,
    body: &[Stmt],
    stmt: &Stmt,
    literal: &ast::ExprStringLiteral,
) -> Option<Edit> {
    let [part] = literal.value.as_slice() else {
        return None;
    };
    if body.iter().all(is_docstring_stmt)
        || !checker.is_alone_on_lines(stmt.range())
        || checker.trailing_comment(stmt).is_some()
    {
        return None;
    }
    let content = checker.text(part.content_range());
    if content.contains('\\') {
        return None;
    }
    let indent = checker.indentation(stmt);
    // The text after the opening quotes starts on the first line, so only the lines below it share an indentation.
    let lines: Vec<String> = match content.split_once('\n') {
        Some((first, rest)) => {
            std::iter::once(first).chain(dedent(rest).split('\n')).map(|line| line.trim_end().to_owned()).collect()
        }
        None => vec![content.trim_end().to_owned()],
    };
    let width = checker.settings.from_ruff.comment_width.checked_sub(indent.chars().count() + 2)?;
    if lines
        .iter()
        .any(|line| line.starts_with('!') || line.chars().count() > width || is_tool_directive(&format!("# {line}")))
    {
        return None;
    }
    let comments: Vec<String> =
        lines.iter().map(|line| if line.is_empty() { "#".to_owned() } else { format!("# {line}") }).collect();
    let eol = checker.line_ending();
    Some(Edit::range_replacement(
        format!("{indent}{}{eol}", comments.join(&format!("{eol}{indent}"))),
        checker.source.full_lines_range(stmt.range()),
    ))
}
