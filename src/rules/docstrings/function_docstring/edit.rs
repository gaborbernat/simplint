use ruff_diagnostics::Edit;
use ruff_python_ast::Stmt;
use ruff_python_ast::token::TokenKind;
use ruff_python_trivia::{SuppressionKind as FormatSuppression, has_leading_content, has_trailing_content};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rules::comments::is_tool_directive;
use crate::rules::docstrings::function_docstring::text::comment_lines;
use crate::rules::docstrings::{docstring_deletion, is_directive_comment};
use crate::rules::edits::{Blanks, statement_lines};

// The level of the block that starts after each indent or dedent token, by where the token starts, the module's being 1.
pub(super) fn block_levels(checker: &Checker) -> Vec<(TextSize, usize)> {
    let mut level: usize = 1;
    let mut levels = Vec::new();
    for token in checker.parsed.tokens() {
        match token.kind() {
            TokenKind::Indent => level += 1,
            TokenKind::Dedent => level = level.saturating_sub(1),
            _ => continue,
        }
        levels.push((token.start(), level));
    }
    levels
}

pub(super) fn removal(checker: &Checker, stmt: &Stmt, owner_body: &[Stmt]) -> Option<Edit> {
    // `ruff format` puts a body on lines of its own, so the fix must not depend on whether it already is. A pragma or
    // tool directive after the docstring, such as `# noqa: E501`, covers its line only and goes with it.
    if owner_body.len() == 1 {
        return Some(Edit::range_replacement("...".to_owned(), with_directive(checker, stmt)));
    }
    docstring_deletion(checker, owner_body)
}

// The comment goes where the docstring was, on lines of its own as `ruff format` would leave the body, wrapped to the
// width its lines get once formatted, so the result does not depend on the layout.
pub(super) fn comment_replacement(
    checker: &Checker,
    owner: TextRange,
    owner_body: &[Stmt],
    lines: &[String],
    levels: &[(TextSize, usize)],
) -> Option<Edit> {
    let source = checker.source;
    let stmt = &owner_body[0];
    let on_header = has_leading_content(stmt.start(), source);
    let indent = if on_header {
        format!("{}{}", checker.indentation(owner), checker.stylist.indentation().as_str())
    } else {
        checker.indentation(stmt).to_owned()
    };
    // The dedents closing the blocks before the owner sit at its start.
    let before = &levels[..levels.partition_point(|(at, _)| *at <= owner.start())];
    let level = before.last().map_or(1, |(_, level)| *level);
    let from_ruff = &checker.settings.from_ruff;
    let room = from_ruff.comment_width.saturating_sub(from_ruff.indent_width * level + 2);
    let comments: Vec<String> = comment_lines(lines, room)
        .into_iter()
        .map(|line| if line.is_empty() { "#".to_owned() } else { format!("# {line}") })
        .collect();
    // A line such as `type: ignore ...` or `fmt: off`, or one holding `# noqa` in its text, would turn into a directive
    // for a tool. Ruff's line length rules let a docstring line of a single word run long, but not a comment line
    // unless it ends in a URL. Where `ruff format` leaves the code alone, an indentation Ruff's E114 rejects for
    // comments stays.
    if comments.iter().any(|comment| {
        comment.match_indices('#').any(|(start, _)| is_tool_directive(&comment[start..]))
            || comment.chars().count() > room + 2
                && !comment.split_whitespace().last().is_some_and(|word| word.contains("://"))
    }) || indent.chars().count() % from_ruff.indent_width.max(1) != 0 && is_left_alone(checker, stmt)
    {
        return None;
    }
    let newline = format!("{}{indent}", checker.line_ending());
    let block = comments.join(&newline);
    let start = if on_header { TextSize::of(source[..stmt.start().to_usize()].trim_end()) } else { stmt.start() };
    let prefix = if on_header { newline.as_str() } else { "" };
    let (end, suffix) = match owner_body.get(1) {
        None => (with_directive(checker, stmt).end(), format!("{newline}...")),
        // A statement after a `;` moves to a line of its own, unless a comment between them would go with it.
        Some(next) if has_trailing_content(stmt.end(), source) => {
            if checker.has_comment_in(TextRange::new(stmt.end(), next.start())) {
                return None;
            }
            (next.start(), newline.clone())
        }
        // Up to the next statement's line, as deleting the docstring would, so a fix removing that statement in the
        // same pass touches this one and waits, rather than leaving a body of comments only.
        Some(_) => {
            let comment = checker
                .trailing_comment(stmt)
                .filter(|comment| !is_directive_comment(checker, *comment))
                .map_or_else(String::new, |comment| format!("  {}", checker.text(comment)));
            (
                statement_lines(source, stmt.range(), Blanks::Trailing).end(),
                format!("{comment}{}", checker.line_ending()),
            )
        }
    };
    Some(Edit::range_replacement(format!("{prefix}{block}{suffix}"), TextRange::new(start, end)))
}

// Whether `ruff format` leaves the statement as it is: its settings exclude the file, the statement ends in
// `# fmt: skip`, or it follows an own-line `# fmt: off` with no `# fmt: on` between.
fn is_left_alone(checker: &Checker, stmt: &Stmt) -> bool {
    let source = checker.source;
    let suppression = |comment: &TextRange| FormatSuppression::from_comment(checker.text(*comment));
    let mut off = false;
    for comment in checker.comment_ranges().iter().take_while(|comment| comment.start() < stmt.start()) {
        if has_leading_content(comment.start(), source) {
            continue;
        }
        match suppression(comment) {
            Some(FormatSuppression::Off) => off = true,
            Some(FormatSuppression::On) => off = false,
            _ => {}
        }
    }
    off || !checker.settings.from_ruff.formatted
        || checker.trailing_comment(stmt).and_then(|comment| suppression(&comment)) == Some(FormatSuppression::Skip)
}

fn with_directive(checker: &Checker, stmt: &Stmt) -> TextRange {
    let directive = checker.trailing_comment(stmt).filter(|comment| is_directive_comment(checker, *comment));
    stmt.range().cover(directive.unwrap_or(stmt.range()))
}
