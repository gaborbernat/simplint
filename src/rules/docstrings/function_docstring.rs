use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::Stmt;
use ruff_python_ast::token::TokenKind;
use ruff_python_trivia::textwrap::dedent;
use ruff_python_trivia::{
    SuppressionKind as FormatSuppression, has_leading_content, has_trailing_content, is_pragma_comment,
};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::PublicApi;
use crate::rules::{docstring, indentation, lines_with_trailing_blanks, statement_deletion};

pub fn function_docstring(checker: &Checker) {
    let body = checker.body();
    let api = PublicApi::of(checker, body);
    let owners = checker
        .function_defs()
        .iter()
        .filter(|function| !api.covers(function.range))
        .map(|function| (function.range, function.body.as_slice(), "Function"))
        .chain(
            checker
                .class_defs()
                .iter()
                .filter(|class| !api.covers(class.range))
                .map(|class| (class.range, class.body.as_slice(), "Class")),
        );
    for (owner, owner_body, kind) in owners {
        let Some(literal) = docstring(owner_body) else {
            continue;
        };
        let stmt = &owner_body[0];
        // The text as written, since an escape such as `\n` would otherwise end the comment line.
        let written: String = literal.value.iter().map(|part| checker.text(part.content_range())).collect();
        let lines = cleaned_lines(&written);
        // One short line rarely says more than the name; anything longer may state a contract the code does not show,
        // such as a tie-break rule, so its text stays as a comment.
        let restates = lines.len() <= 1 && lines.iter().all(|line| line.split_whitespace().count() <= 5);
        let (fix, title) = if restates {
            (removal(checker, stmt, owner_body).map(Fix::unsafe_edit), "Remove the docstring")
        } else {
            (
                comment_replacement(checker, owner, owner_body, &lines).map(Fix::unsafe_edit),
                "Turn the docstring into a comment",
            )
        };
        checker.report_fix(
            Rule::FunctionDocstring,
            stmt.range(),
            format!("{kind} docstring on internal code; let the name, signature and types document it"),
            fix,
            title,
        );
    }
}

fn removal(checker: &Checker, stmt: &Stmt, owner_body: &[Stmt]) -> Option<Edit> {
    // `ruff format` puts a body on lines of its own, so the fix must not depend on whether it already is. A pragma
    // after the docstring, such as `# noqa: E501`, covers its line only and goes with it.
    if owner_body.len() == 1 {
        return Some(Edit::range_replacement("...".to_owned(), with_pragma(checker, stmt)));
    }
    statement_deletion(checker, stmt.range(), owner_body.get(1).map(Ranged::range), true).map(Edit::range_deletion)
}

// The comment goes where the docstring was, on lines of its own as `ruff format` would leave the body, wrapped to the
// width its lines get once formatted, so the result does not depend on the layout.
fn comment_replacement(checker: &Checker, owner: TextRange, owner_body: &[Stmt], lines: &[String]) -> Option<Edit> {
    let source = checker.source;
    let stmt = &owner_body[0];
    let on_header = has_leading_content(stmt.start(), source);
    let indent = if on_header {
        format!("{}{}", indentation(source, owner), checker.stylist.indentation().as_str())
    } else {
        indentation(source, stmt).to_owned()
    };
    // The dedents closing the blocks before the owner sit at its start.
    let level = checker.parsed.tokens().iter().take_while(|token| token.start() <= owner.start()).fold(
        1,
        |level: usize, token| match token.kind() {
            TokenKind::Indent => level + 1,
            TokenKind::Dedent => level.saturating_sub(1),
            _ => level,
        },
    );
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
        comment.match_indices('#').any(|(start, _)| is_directive(&comment[start..]))
            || comment.chars().count() > room + 2
                && !comment.split_whitespace().last().is_some_and(|word| word.contains("://"))
    }) || indent.chars().count() % from_ruff.indent_width.max(1) != 0 && is_left_alone(checker, stmt)
    {
        return None;
    }
    let newline = format!("{}{indent}", checker.stylist.line_ending().as_str());
    let block = comments.join(&newline);
    let start = if on_header { TextSize::of(source[..stmt.start().to_usize()].trim_end()) } else { stmt.start() };
    let prefix = if on_header { newline.as_str() } else { "" };
    let (end, suffix) = match owner_body.get(1) {
        None => (with_pragma(checker, stmt).end(), format!("{newline}...")),
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
                .filter(|comment| !is_pragma_comment(checker.text(*comment)))
                .map_or_else(String::new, |comment| format!("  {}", checker.text(comment)));
            (
                lines_with_trailing_blanks(source, stmt.range()).end(),
                format!("{comment}{}", checker.stylist.line_ending().as_str()),
            )
        }
    };
    Some(Edit::range_replacement(format!("{prefix}{block}{suffix}"), TextRange::new(start, end)))
}

fn is_directive(comment: &str) -> bool {
    let text = comment.trim_start_matches('#').trim_start();
    is_pragma_comment(comment) || text.starts_with("fmt:") || text.starts_with("simplint:")
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

fn with_pragma(checker: &Checker, stmt: &Stmt) -> TextRange {
    let pragma = checker.trailing_comment(stmt).filter(|comment| is_pragma_comment(checker.text(*comment)));
    stmt.range().cover(pragma.unwrap_or(stmt.range()))
}

// The docstring's text as `inspect.cleandoc` gives it, without trailing whitespace and without the underlines of reST
// and numpydoc headings, which as comments would read as section dividers.
fn cleaned_lines(text: &str) -> Vec<String> {
    let text = text.replace('\t', "        ");
    let (first, rest) = text.split_once('\n').unwrap_or((&text, ""));
    let mut lines: Vec<String> = std::iter::once(first.trim().to_owned())
        .chain(dedent(rest).lines().map(|line| line.trim_end().to_owned()))
        .filter(|line| !is_underline(line))
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let leading = lines.iter().take_while(|line| line.is_empty()).count();
    lines.drain(..leading);
    lines
}

fn is_underline(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|char| matches!(char, '-' | '=' | '~' | '^' | '*' | '+' | '#' | '_'))
}

// Lines that fit are kept as written, so lists and indented blocks keep their shape. What a long line wraps over joins
// the next line when that line goes on with the same sentence, rather than leaving a word on a line of its own.
fn comment_lines(lines: &[String], room: usize) -> Vec<String> {
    let mut comments = Vec::new();
    let mut carry = String::new();
    for (index, line) in lines.iter().enumerate() {
        let lead = &line[..line.len() - line.trim_start().len()];
        let line = if carry.is_empty() { line.clone() } else { format!("{lead}{carry} {}", line.trim_start()) };
        let mut parts = wrapped(&line, room);
        carry = if parts.len() > 1 && lines.get(index + 1).is_some_and(|next| continues(next, lead)) {
            parts.pop().unwrap_or_default().trim_start().to_owned()
        } else {
            String::new()
        };
        comments.extend(parts);
    }
    comments
}

// A line at the same indentation starting in lowercase goes on with a sentence; one opening with a field name such as
// `name:` or `name (int):` starts an item of its own.
fn continues(line: &str, lead: &str) -> bool {
    line.strip_prefix(lead).is_some_and(|text| {
        text.starts_with(char::is_lowercase) && !text.split_whitespace().take(2).any(|word| word.ends_with(':'))
    })
}

// Splits a line too long for `room` at spaces, keeping its indentation on each part; a word longer than `room`, such as
// a URL, keeps a line of its own.
fn wrapped(line: &str, room: usize) -> Vec<String> {
    if line.chars().count() <= room {
        return vec![line.to_owned()];
    }
    let lead = &line[..line.len() - line.trim_start().len()];
    let mut parts = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > room {
            parts.push(std::mem::take(&mut current));
        }
        if current.is_empty() {
            current.push_str(lead);
            current.push_str(word);
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    parts.push(current);
    parts
}
