use ruff_python_trivia::textwrap::dedent;

use crate::rst::{field_marker, indent_width};
use crate::rules::docstrings::rst::SIGNATURE_SECTIONS;

// The docstring's text as `inspect.cleandoc` gives it, without trailing whitespace and without the underlines of rst
// and numpydoc headings, which as comments would read as section dividers.
pub(super) fn cleaned_lines(text: &str) -> Vec<String> {
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

pub(super) fn drop_fields(lines: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(end) = field_block_end(lines, index) {
            index = end;
            continue;
        }
        let line = &lines[index];
        if !(line.is_empty() && kept.last().is_none_or(String::is_empty)) {
            kept.push(line.clone());
        }
        index += 1;
    }
    while kept.last().is_some_and(String::is_empty) {
        kept.pop();
    }
    kept
}

fn field_block_end(lines: &[String], start: usize) -> Option<usize> {
    let line = &lines[start];
    let trimmed = line.trim();
    let lead = indent_width(line);
    if field_marker(trimmed).is_some() {
        return Some(indented_end(lines, start, lead));
    }
    let (name, colon) = match trimmed.strip_suffix(':') {
        Some(name) => (name, true),
        None => (trimmed, false),
    };
    if !SIGNATURE_SECTIONS.contains(&name) {
        return None;
    }
    if colon {
        return Some(indented_end(lines, start, lead));
    }
    // A numpydoc section lists its names at the header's own indentation, so its body runs up to a blank line or the
    // next section.
    let body = lines[start + 1..]
        .iter()
        .position(|line| line.trim().is_empty() || SIGNATURE_SECTIONS.contains(&line.trim()))
        .unwrap_or(lines.len() - start - 1);
    Some(start + 1 + body)
}

fn indented_end(lines: &[String], start: usize, lead: usize) -> usize {
    let mut end = start + 1;
    for (offset, line) in lines[start + 1..].iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if indent_width(line) <= lead {
            break;
        }
        end = start + 2 + offset;
    }
    end
}

// Lines that fit are kept as written, so lists and indented blocks keep their shape. What a long line wraps over joins
// the next line when that line goes on with the same sentence, rather than leaving a word on a line of its own.
pub(super) fn comment_lines(lines: &[String], room: usize) -> Vec<String> {
    let mut comments = Vec::new();
    let mut carry = String::new();
    for (index, line) in lines.iter().enumerate() {
        let lead = &line[..indent_width(line)];
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

// A word longer than `room`, such as a URL, keeps a line of its own.
fn wrapped(line: &str, room: usize) -> Vec<String> {
    if line.chars().count() <= room {
        return vec![line.to_owned()];
    }
    let lead = &line[..indent_width(line)];
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

// A line at the same indentation starting in lowercase goes on with a sentence; one opening with a field name such as
// `name:` or `name (int):` starts an item of its own.
fn continues(line: &str, lead: &str) -> bool {
    line.strip_prefix(lead).is_some_and(|text| {
        text.starts_with(char::is_lowercase) && !text.split_whitespace().take(2).any(|word| word.ends_with(':'))
    })
}
