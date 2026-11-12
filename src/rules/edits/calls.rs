use ruff_python_ast as ast;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;

// The argument at `range`, a keyword or a parenthesized positional argument, with the comma that separates it from a
// neighbor, so the rest of the call keeps its layout. A sole argument goes with its trailing comma, up to the closing
// parenthesis.
pub(crate) fn argument_removal(checker: &Checker, call: &ast::ExprCall, range: TextRange) -> Option<TextRange> {
    if checker.has_comment_in(call.range()) {
        return None;
    }
    let mut arguments: Vec<TextRange> = call
        .arguments
        .args
        .iter()
        .map(|argument| checker.parenthesized_range(argument, (&call.arguments).into()))
        .chain(call.arguments.keywords.iter().map(Ranged::range))
        .collect();
    arguments.sort_by_key(Ranged::start);
    let index = arguments.iter().position(|argument| *argument == range)?;
    Some(match (index.checked_sub(1).map(|previous| arguments[previous]), arguments.get(index + 1)) {
        (_, Some(next)) => TextRange::new(range.start(), next.start()),
        (Some(previous), None) => TextRange::new(previous.end(), range.end()),
        (None, None) => TextRange::new(range.start(), call.range().end() - TextSize::from(1)),
    })
}
