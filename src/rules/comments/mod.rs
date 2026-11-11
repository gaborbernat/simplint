mod blanket_ignore;
mod directive;
mod long_paragraph;
mod numeric_ignore;
mod restating_comment;
mod section_comment;
mod string_statement_comment;
mod suppression_without_reason;

use ruff_python_ast::Stmt;
use ruff_source_file::LineRanges;
use ruff_text_size::TextRange;

use crate::lint::checker::Checker;

pub(crate) use blanket_ignore::blanket_ignore;
pub(crate) use directive::{Comment, is_tool_directive};
pub(crate) use long_paragraph::long_paragraph;
pub(crate) use numeric_ignore::numeric_ignore;
pub(crate) use restating_comment::restating_comment;
pub(crate) use section_comment::section_comment;
pub(crate) use string_statement_comment::string_statement_comment;
pub(crate) use suppression_without_reason::suppression_without_reason;

// Whether a body's owner is a function or a class, the bodies whose first string is a docstring; a module's is too.
pub(crate) fn owns_docstring(owner: Option<&Stmt>) -> bool {
    owner.is_none_or(|owner| matches!(owner, Stmt::FunctionDef(_) | Stmt::ClassDef(_)))
}

// The own-line comments of the file in runs, where a comment joins the run before it when that one ends on the line
// above. A comment with code before it on its line sits on a line of code, so it never joins a run.
pub(crate) fn own_line_comment_runs(checker: &Checker) -> Vec<Vec<TextRange>> {
    let source = checker.source;
    checker
        .state()
        .own_line_comments(checker)
        .chunk_by(|previous, next| follows(source, *previous, *next))
        .map(<[TextRange]>::to_vec)
        .collect()
}

// Whether `next` starts on the line below the one `previous` ends on.
pub(crate) fn follows(source: &str, previous: TextRange, next: TextRange) -> bool {
    source.full_line_end(previous.end()) == source.line_start(next.start())
}
