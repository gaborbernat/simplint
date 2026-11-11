use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::{self as ast, Stmt, StringFlags};
use ruff_python_trivia::textwrap::dedent;
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::{is_tool_directive, owns_docstring};
use crate::rules::{indentation, is_alone_on_lines};
use crate::semantic::future_import;
use crate::syntax::for_each_body;

pub fn string_statement_comment(checker: &Checker) {
    // The compiler checks where a `from __future__` import sits against the statements before it, and a string is one.
    let has_future_import = checker.body().iter().any(|stmt| future_import(stmt).is_some());
    for_each_body(checker.body(), &mut |body, owner| {
        for (index, stmt) in body.iter().enumerate() {
            let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
                continue;
            };
            // Triple quotes mark a block of text; a short quoted string is more often a marker, such as a line a
            // tracer or a test expects to run.
            let is_text_block = value
                .as_string_literal_expr()
                .is_some_and(|literal| literal.value.iter().all(|part| part.flags.is_triple_quoted()));
            if !is_text_block || is_docstring(index, owner) || is_attribute_docstring(body, index) {
                continue;
            }
            let fix = (!has_future_import)
                .then(|| comment_replacement(checker, body, stmt, value.as_string_literal_expr()))
                .flatten();
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

// The first statement of a module, class or function body, which Python keeps as its docstring.
fn is_docstring(index: usize, owner: Option<&Stmt>) -> bool {
    index == 0 && owns_docstring(owner)
}

// A string after an assignment documents the attribute it assigns, as Sphinx reads it.
fn is_attribute_docstring(body: &[Stmt], index: usize) -> bool {
    index > 0 && matches!(body[index - 1], Stmt::Assign(_) | Stmt::AnnAssign(_))
}

// One comment line per line of the string, so the code after it keeps its line numbers. The strings of a body that
// holds nothing else cannot all become comments, which would leave the body empty; a string sharing its line with code
// or a comment, an escape the compiler reads, a text that would start a tool directive, or a line past the comment
// width stays.
fn comment_replacement(
    checker: &Checker,
    body: &[Stmt],
    stmt: &Stmt,
    literal: Option<&ast::ExprStringLiteral>,
) -> Option<Edit> {
    let source = checker.source;
    let [part] = literal?.value.as_slice() else {
        return None;
    };
    if body.iter().all(is_docstring_stmt)
        || !is_alone_on_lines(source, stmt.range())
        || checker.trailing_comment(stmt).is_some()
    {
        return None;
    }
    let content = checker.text(part.content_range());
    if content.contains('\\') {
        return None;
    }
    let indent = indentation(source, stmt);
    let width = checker.settings.from_ruff.comment_width.checked_sub(indent.chars().count() + 2)?;
    // The text after the opening quotes starts on the first line, so only the lines below it share an indentation.
    let lines: Vec<String> = match content.split_once('\n') {
        Some((first, rest)) => {
            let rest = dedent(rest);
            std::iter::once(first).chain(rest.split('\n')).map(|line| line.trim_end().to_owned()).collect()
        }
        None => vec![content.trim_end().to_owned()],
    };
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
        source.full_lines_range(stmt.range()),
    ))
}
