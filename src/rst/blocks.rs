use super::fields::field_marker;
use super::lines::{Line, TAB_WIDTH, columns, docstring_lines};

pub(crate) struct Classified<'a> {
    pub(crate) line: Line<'a>,
    // Whether the line is not prose: literal or doctest code, a comment, a hyperlink target, a section underline, a
    // transition, or the content of a `raw` or `math` directive.
    pub(crate) code: bool,
    // Whether the line starts a block of its own, so an inline literal left open above it closes there.
    pub(crate) opens: bool,
    // Whether the text is not read as markup at all: a literal block, doctest, comment or target, or the content of a
    // verbatim directive. A Markdown fence in other code, such as a `~~~` underline, is still a fence.
    pub(crate) verbatim: bool,
}

// How a line reads: as prose, as code, or as text that is not read as markup at all.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(super) enum Shown {
    #[default]
    Prose,
    Code,
    Verbatim,
}

#[derive(Clone, Copy, Default)]
pub(super) struct Mark {
    pub(super) shown: Shown,
    pub(super) opens: bool,
    // Whether the line is paragraph text of the docstring's top level, which a field right under it joins.
    pub(super) paragraph: bool,
}

pub(crate) fn classify_docstring(text: &str) -> Vec<Classified<'_>> {
    let lines = docstring_lines(text);
    let marks = marks(&lines);
    lines
        .into_iter()
        .zip(marks)
        .map(|(line, mark)| Classified {
            line,
            code: mark.shown != Shown::Prose,
            opens: mark.opens,
            verbatim: mark.shown == Shown::Verbatim,
        })
        .collect()
}

// A block's body is a range of the one line vector, and a list item or field body replaces only the text of its first
// line. Nesting therefore costs no copies, and memory stays linear in the number of lines however deep they nest.
pub(super) fn marks(lines: &[Line<'_>]) -> Vec<Mark> {
    let base = lines.iter().filter(|line| !line.text.is_empty()).map(|line| line.indent).min().unwrap_or_default();
    let mut out = vec![Mark::default(); lines.len()];
    let mut pending = vec![Body { start: 0, end: lines.len(), base, first: None, top: true }];
    while let Some(body) = pending.pop() {
        Scan::new(lines, &mut out, &mut pending, body).run();
    }
    out
}

#[derive(Clone, Copy)]
struct Body<'a> {
    start: usize,
    end: usize,
    // The indentation the body's blocks start at, in columns.
    base: usize,
    // The line the body starts with, when a list item or field replaces its marker's text with the text after it.
    first: Option<Line<'a>>,
    // Whether the body is the docstring itself, the only level where section titles are read.
    top: bool,
}

struct Scan<'s, 'l, 'a> {
    lines: &'l [Line<'a>],
    out: &'s mut [Mark],
    pending: &'s mut Vec<Body<'a>>,
    body: Body<'a>,
    // Whether the previous line is paragraph text of this body, with no blank line since.
    paragraph: bool,
    // Whether that paragraph ends in `::`, and how many lines it has.
    literal_marker: bool,
    paragraph_lines: usize,
    // The first blank line after a paragraph that ends in `::`, where a literal block may begin.
    literal_from: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Sequence {
    Arabic,
    LowerAlpha,
    UpperAlpha,
    LowerRoman,
    UpperRoman,
    Auto,
}

const NUMERALS: [(&str, usize); 13] = [
    ("m", 1000),
    ("cm", 900),
    ("d", 500),
    ("cd", 400),
    ("c", 100),
    ("xc", 90),
    ("l", 50),
    ("xl", 40),
    ("x", 10),
    ("ix", 9),
    ("v", 5),
    ("iv", 4),
    ("i", 1),
];

impl<'s, 'l, 'a> Scan<'s, 'l, 'a> {
    fn new(lines: &'l [Line<'a>], out: &'s mut [Mark], pending: &'s mut Vec<Body<'a>>, body: Body<'a>) -> Self {
        Self {
            lines,
            out,
            pending,
            body,
            paragraph: false,
            literal_marker: false,
            paragraph_lines: 0,
            literal_from: None,
        }
    }

    fn run(&mut self) {
        let Body { start, end, first, .. } = self.body;
        if first.is_some() {
            self.out[start].opens = true;
        }
        let mut index = start;
        while index < end {
            index = self.block(index);
        }
    }

    // The line at `index`, with the replaced text when it is the body's first line.
    fn line(&self, index: usize) -> Line<'a> {
        match self.body.first {
            Some(first) if index == self.body.start => first,
            _ => self.lines[index],
        }
    }

    // Classifies the block that starts at `index` and returns where the next block starts.
    fn block(&mut self, index: usize) -> usize {
        let line = self.line(index);
        let base = self.body.base;
        if line.text.is_empty() {
            if self.paragraph {
                self.literal_from = self.literal_marker.then_some(index);
            }
            self.paragraph = false;
            return index + 1;
        }
        if let Some(from) = self.literal_from.take() {
            if line.indent > base {
                let end = self.region_end(index);
                self.verbatim(from, end);
                return end;
            }
            if let Some(end) = self.quoted_end(index) {
                self.verbatim(index, end);
                return end;
            }
        }
        if self.paragraph {
            if line.indent == base {
                self.out[index] = Mark { paragraph: self.body.top, ..Mark::default() };
                self.literal_marker = ends_with_marker(line.text);
                self.paragraph_lines += 1;
                return index + 1;
            }
            // Docutils reads an indented line right under a paragraph of several lines as a literal block, and under
            // a single line as a definition.
            if self.literal_marker && self.paragraph_lines > 1 {
                let end = self.region_end(index);
                self.verbatim(index, end);
                self.paragraph = false;
                return end;
            }
            return self.nested(index);
        }
        if line.indent > base {
            return self.nested(index);
        }
        let trimmed = line.text.trim_start();
        if let Some(marker) = field_marker(trimmed) {
            return self.field(index, trimmed[marker.len() + 2..].trim_start());
        }
        if let Some(len) = bullet_len(trimmed).or_else(|| self.enumerator_len(index, trimmed)) {
            let rest = trimmed[len..].trim_start();
            return self.list_item(index, rest, &trimmed[..trimmed.len() - rest.len()]);
        }
        if trimmed == ".." || trimmed.starts_with(".. ") {
            return self.explicit(index, trimmed);
        }
        if trimmed == "__" || trimmed.starts_with("__ ") {
            return self.non_prose(index, trimmed);
        }
        if trimmed.strip_prefix(">>>").is_some_and(|rest| rest.is_empty() || rest.starts_with(' ')) {
            return self.doctest(index);
        }
        // Docutils reads a line block, which starts with a bar, before it reads a title or transition.
        if self.body.top
            && !(trimmed == "|" || trimmed.starts_with("| "))
            && let Some(end) = self.heading(index)
        {
            return end;
        }
        self.paragraph = true;
        self.out[index] = Mark { paragraph: self.body.top, opens: true, ..Mark::default() };
        self.literal_marker = ends_with_marker(line.text);
        self.paragraph_lines = 1;
        index + 1
    }

    // A field: its body is the deeper lines after it, measured from the least indented of them.
    fn field(&mut self, index: usize, rest: &'a str) -> usize {
        let end = self.region_end(index + 1);
        let base = self.min_indent(index + 1, end);
        let first = Line { text: rest, indent: base, ..self.line(index) };
        self.defer(index, end, base, Some(first));
        end
    }

    // A bullet or enumerated item, or a footnote: its body is the lines indented at least to where its text starts,
    // as docutils reads a list item.
    fn list_item(&mut self, index: usize, rest: &'a str, prefix: &str) -> usize {
        let column = self.line(index).indent + columns(prefix, TAB_WIDTH);
        let end = self.body_end(index + 1, column);
        let first = Line { text: rest, indent: column, ..self.line(index) };
        self.defer(index, end, column, Some(first));
        end
    }

    // Blocks indented under a block that is not a list item, field or directive: a block quote, or the definition under
    // a term.
    fn nested(&mut self, index: usize) -> usize {
        self.paragraph = false;
        let end = self.region_end(index);
        let base = self.min_indent(index, end);
        self.defer(index, end, base, None);
        end
    }

    fn defer(&mut self, start: usize, end: usize, base: usize, first: Option<Line<'a>>) {
        self.pending.push(Body { start, end, base, first, top: false });
    }

    fn explicit(&mut self, index: usize, trimmed: &'a str) -> usize {
        if let Some(rest) = trimmed.strip_prefix(".. |") {
            return match rest.split_once('|').and_then(|(_, after)| directive_name(after)) {
                Some(name) => self.directive(index, name, true),
                None => self.non_prose(index, trimmed),
            };
        }
        if let Some(name) = trimmed.strip_prefix(".. ").and_then(directive_name) {
            return self.directive(index, name, false);
        }
        if let Some(rest) = trimmed.strip_prefix(".. ")
            && let Some(close) = note_label_end(rest)
        {
            let body = rest[close..].trim_start();
            return self.list_item(index, body, &trimmed[..trimmed.len() - body.len()]);
        }
        self.non_prose(index, trimmed)
    }

    // A directive's content is code or prose by the directive's name. A substitution definition's line is not prose
    // itself, which is how it differs from an ordinary directive's line.
    fn directive(&mut self, index: usize, name: &str, substitution: bool) -> usize {
        // The content of these directives is code, or is read as it is written, not as prose.
        const VERBATIM: &[&str] = &[
            "code-block",
            "code",
            "sourcecode",
            "doctest",
            "testcode",
            "testoutput",
            "ipython",
            "jupyter-execute",
            "raw",
            "math",
        ];
        let end = self.region_end(index + 1);
        let shown = if substitution { Shown::Code } else { Shown::Prose };
        self.out[index] = Mark { shown, opens: true, paragraph: false };
        if VERBATIM.contains(&name) {
            self.verbatim(index + 1, end);
        } else {
            let base = self.min_indent(index + 1, end);
            self.defer(index + 1, end, base, None);
        }
        end
    }

    // Comments, hyperlink targets and anonymous targets: their text is not prose, nor is the block under them.
    fn non_prose(&mut self, index: usize, trimmed: &str) -> usize {
        // A `..` with nothing after it and a blank line below is an empty comment, which leaves the block after it
        // alone.
        let empty = trimmed[2..].trim().is_empty();
        let end = if empty && (index + 1 >= self.body.end || self.line(index + 1).text.is_empty()) {
            index + 1
        } else {
            self.region_end(index + 1)
        };
        self.verbatim(index, end);
        self.out[index].opens = true;
        end
    }

    fn doctest(&mut self, index: usize) -> usize {
        let end = (index..self.body.end).find(|&line| self.lines[line].text.is_empty()).unwrap_or(self.body.end);
        self.verbatim(index, end);
        self.out[index].opens = true;
        end
    }

    // A section title or transition at the top level of a docstring. Docutils reads its title text as inline markup,
    // but its underline and overline are not text.
    // A short line that is punctuation alone is read as an overline first, and then again as text when no title
    // follows, so the same line can still start a title with an underline below it.
    fn heading(&mut self, index: usize) -> Option<usize> {
        self.overline_title(index).or_else(|| self.text_title(index))
    }

    // A title with an underline below it, when the underline is at least as long as the title or as long as four
    // characters.
    fn text_title(&mut self, index: usize) -> Option<usize> {
        let under_index = index + 1;
        if under_index >= self.body.end {
            return None;
        }
        let under = self.line(under_index);
        let marker = underline(under.text)?;
        if under.indent != self.body.base {
            return None;
        }
        let length = marker.chars().count();
        if self.line(index).text.trim().chars().count() > length && length < 4 {
            return None;
        }
        self.out[index] = Mark { opens: true, ..Mark::default() };
        self.code(under_index, under_index + 1);
        self.out[under_index].opens = true;
        Some(index + 2)
    }

    // A transition, or a title with an overline and an underline that match. An overline shorter than four characters
    // and not matched by an underline is ordinary text.
    fn overline_title(&mut self, index: usize) -> Option<usize> {
        let over = underline(self.line(index).text)?;
        let long = over.chars().count() >= 4;
        let has_title = index + 1 < self.body.end && !self.line(index + 1).text.is_empty();
        if !has_title {
            if !long {
                return None;
            }
            self.code(index, index + 1);
            self.out[index].opens = true;
            return Some(index + 1);
        }
        if index + 2 >= self.body.end {
            if !long {
                return None;
            }
            self.code(index, index + 2);
            self.out[index].opens = true;
            return Some(index + 2);
        }
        let under = self.line(index + 2);
        if under.indent == self.body.base && underline(under.text) == Some(over) {
            self.code(index, index + 1);
            self.out[index].opens = true;
            self.out[index + 1] = Mark { opens: true, ..Mark::default() };
            self.code(index + 2, index + 3);
            self.out[index + 2].opens = true;
            return Some(index + 3);
        }
        if !long {
            return None;
        }
        self.code(index, index + 3);
        self.out[index].opens = true;
        Some(index + 3)
    }

    // The enumerator of an enumerated item, when docutils reads it as one: its ordinal is valid for its sequence, and
    // the next line is blank, indented, or the next enumerator.
    fn enumerator_len(&self, index: usize, trimmed: &str) -> Option<usize> {
        let parenthesized = trimmed.starts_with('(');
        let body = &trimmed[usize::from(parenthesized)..];
        let token = body.find(|char: char| !(char.is_ascii_alphanumeric() || char == '#')).unwrap_or(body.len());
        let delimiter = body[token..].chars().next()?;
        if !matches!((parenthesized, delimiter), (true, ')') | (false, ')' | '.')) {
            return None;
        }
        let len = usize::from(parenthesized) + token + 1;
        let rest = &trimmed[len..];
        if !(rest.is_empty() || rest.starts_with(' ')) {
            return None;
        }
        let (sequence, ordinal) = enumerator(&body[..token])?;
        self.next_enumerator_follows(index, (parenthesized, delimiter), sequence, ordinal).then_some(len)
    }

    fn next_enumerator_follows(
        &self,
        index: usize,
        (parenthesized, delimiter): (bool, char),
        sequence: Sequence,
        ordinal: usize,
    ) -> bool {
        if index + 1 >= self.body.end {
            return true;
        }
        let next = self.line(index + 1);
        if next.text.is_empty() || next.indent > self.body.base {
            return true;
        }
        let following = ordinal + 1;
        let marker = match sequence {
            Sequence::Auto => Some("#".to_owned()),
            Sequence::Arabic => Some(following.to_string()),
            Sequence::LowerAlpha => letter(following, b'a'),
            Sequence::UpperAlpha => letter(following, b'A'),
            Sequence::LowerRoman => Some(roman_text(following)),
            Sequence::UpperRoman => Some(roman_text(following).to_ascii_uppercase()),
        };
        let Some(marker) = marker else {
            return false;
        };
        let prefix = if parenthesized { "(" } else { "" };
        let text = next.text.trim_start();
        text.starts_with(&format!("{prefix}{marker}{delimiter} "))
            || text.starts_with(&format!("{prefix}#{delimiter} "))
    }

    fn code(&mut self, start: usize, end: usize) {
        for mark in &mut self.out[start..end] {
            if mark.shown == Shown::Prose {
                mark.shown = Shown::Code;
            }
        }
    }

    fn verbatim(&mut self, start: usize, end: usize) {
        for mark in &mut self.out[start..end] {
            mark.shown = Shown::Verbatim;
        }
    }

    fn min_indent(&self, from: usize, end: usize) -> usize {
        (from..end)
            .map(|index| self.line(index))
            .filter(|line| !line.text.is_empty())
            .map(|line| line.indent)
            .min()
            .unwrap_or_default()
    }

    // The first line from `from` that is not indented past the base, so a block's lines end before the next block.
    fn region_end(&self, from: usize) -> usize {
        (from..self.body.end)
            .find(|&index| {
                let line = self.line(index);
                !line.text.is_empty() && line.indent <= self.body.base
            })
            .unwrap_or(self.body.end)
    }

    // The first non-blank line from `from` that is indented less than `column`, where a list item's body ends.
    fn body_end(&self, from: usize, column: usize) -> usize {
        (from..self.body.end)
            .find(|&index| {
                let line = self.lines[index];
                !line.text.is_empty() && line.indent < column
            })
            .unwrap_or(self.body.end)
    }

    fn quoted_end(&self, index: usize) -> Option<usize> {
        let quote = self.line(index).text.trim_start().chars().next().filter(char::is_ascii_punctuation)?;
        let end = (index..self.body.end)
            .find(|&other| {
                let line = self.line(other);
                line.indent != self.body.base || !line.text.trim_start().starts_with(quote)
            })
            .unwrap_or(self.body.end);
        Some(end)
    }
}

// The length of a bullet that opens a list item, such as `- ` or `• `, when a space or the end follows it.
fn bullet_len(trimmed: &str) -> Option<usize> {
    let rest = trimmed.strip_prefix(['-', '+', '*', '\u{2022}', '\u{2023}', '\u{2043}'])?;
    (rest.is_empty() || rest.starts_with(' ')).then_some(trimmed.len() - rest.len())
}

// The sequence and ordinal of an enumerator's text, or `None` when docutils cannot read its ordinal, as with `civil`.
fn enumerator(token: &str) -> Option<(Sequence, usize)> {
    if token == "#" {
        return Some((Sequence::Auto, 1));
    }
    if token == "i" {
        return Some((Sequence::LowerRoman, 1));
    }
    if token == "I" {
        return Some((Sequence::UpperRoman, 1));
    }
    if !token.is_empty() && token.bytes().all(|byte| byte.is_ascii_digit()) {
        return Some((Sequence::Arabic, token.parse().ok()?));
    }
    // A single character that reaches here is a letter, since digits and `#` are read above.
    if let [letter] = token.as_bytes() {
        let (sequence, first) =
            if letter.is_ascii_lowercase() { (Sequence::LowerAlpha, b'a') } else { (Sequence::UpperAlpha, b'A') };
        return Some((sequence, usize::from(letter - first) + 1));
    }
    if token.bytes().all(|byte| b"ivxlcdm".contains(&byte)) {
        return roman_value(token).map(|value| (Sequence::LowerRoman, value));
    }
    if token.bytes().all(|byte| b"IVXLCDM".contains(&byte)) {
        return roman_value(&token.to_ascii_lowercase()).map(|value| (Sequence::UpperRoman, value));
    }
    None
}

// The value of a lowercase Roman numeral, when it is written the way it is rendered.
fn roman_value(text: &str) -> Option<usize> {
    let mut rest = text;
    let mut value = 0;
    while !rest.is_empty() {
        let (symbol, digit) = NUMERALS.iter().find(|(symbol, _)| rest.starts_with(symbol))?;
        value += digit;
        rest = &rest[symbol.len()..];
    }
    (value > 0 && roman_text(value) == text).then_some(value)
}

// The letter of an alphabetic ordinal, `c` for 3 counting from `a`, when the ordinal has one.
fn letter(ordinal: usize, first: u8) -> Option<String> {
    let offset = u8::try_from(ordinal.checked_sub(1)?).ok().filter(|offset| *offset < 26)?;
    Some(char::from(first + offset).to_string())
}

fn roman_text(mut value: usize) -> String {
    let mut text = String::new();
    for (symbol, digit) in NUMERALS {
        while value >= digit {
            text.push_str(symbol);
            value -= digit;
        }
    }
    text
}

// The text of a line that is one punctuation character repeated, the shape of a section underline or a transition.
fn underline(text: &str) -> Option<&str> {
    let line = text.trim();
    let first = line.chars().next()?;
    (first.is_ascii_punctuation() && line.chars().all(|char| char == first)).then_some(line)
}

// The name of the directive that `name:: argument` opens, once the `..` before it is gone.
fn directive_name(text: &str) -> Option<&str> {
    let (name, rest) = text.trim_start().split_once("::")?;
    let name = name.trim_end();
    (!name.is_empty()
        && name.chars().all(|char| char.is_alphanumeric() || "-_.+:".contains(char))
        && (rest.is_empty() || rest.starts_with(' ')))
    .then_some(name)
}

// The end of a footnote or citation label at the start of `text`, such as `[1]`, `[#]`, `[*]` or `[note]`, with the
// space after it. Any other label makes the line a comment.
fn note_label_end(text: &str) -> Option<usize> {
    let close = text.strip_prefix('[')?.find(']')? + 1;
    let label = &text[1..close];
    let after = &text[close + 1..];
    let name = |label: &str| {
        label.starts_with(|char: char| char.is_alphanumeric())
            && label.chars().all(|char| char.is_alphanumeric() || "-_.+:".contains(char))
    };
    let valid = label == "*"
        || (!label.is_empty() && label.chars().all(|char| char.is_ascii_digit()))
        || label.strip_prefix('#').map_or_else(|| name(label), |rest| rest.is_empty() || name(rest));
    (valid && (after.is_empty() || after.starts_with(' '))).then_some(close + 1)
}

// A `::` ends a paragraph into a literal block, unless an odd run of backslashes escapes its first colon.
fn ends_with_marker(text: &str) -> bool {
    text.strip_suffix("::").is_some_and(|rest| rest.bytes().rev().take_while(|&byte| byte == b'\\').count() % 2 == 0)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use test_case::test_case;

    use super::classify_docstring;

    #[test_case("Text.\n", &[false] ; "prose")]
    #[test_case("Example::\n\n    x = 1\n\nAfter\n", &[false, true, true, true, false] ; "literal_block")]
    #[test_case(".. code-block:: python\n\n    x = 1\n\nText\n", &[false, true, true, true, false] ; "code_directive")]
    #[test_case(".. note::\n\n    Body\n", &[false, false, false] ; "prose_directive")]
    #[test_case(">>> f()\n1\n\nText\n", &[true, true, false, false] ; "doctest_block")]
    #[test_case(">>> x::\n\n    y\n", &[true, false, false] ; "doctest_line_does_not_open_a_literal_block")]
    #[test_case("Example::\n\n> one\n> two\n\nAfter\n", &[false, false, true, true, false, false] ; "quoted_block")]
    #[test_case("Example::\n\nafter\n", &[false, false, false] ; "no_quoted_block_for_text")]
    #[test_case("Example\\::\n\n    x\n", &[false, false, false] ; "escaped_double_colon_is_not_a_marker")]
    #[test_case("Example::\n    x\n", &[false, false] ; "no_blank_line_before_the_block")]
    #[test_case("Text\n>>> x\n", &[false, false] ; "doctest_continues_a_paragraph")]
    #[test_case("Summary.\n\n- item::\n\n  not code\n", &[false, false, false, false, false] ; "item_body_indent")]
    #[test_case("Summary.\n\n- Example::\n\n      code ``x\n", &[false, false, false, true, true] ; "bullet_body_is_measured_from_its_text")]
    #[test_case("Summary.\n\n1. Example::\n\n       code ``x\n", &[false, false, false, true, true] ; "enumerated_body_is_measured_from_its_text")]
    #[test_case("Summary.\n\n1) item::\n\n   not code\n", &[false, false, false, false, false] ; "enumerator_body")]
    #[test_case("Summary.\n\n(i) item::\n\n    not code\n", &[false, false, false, false, false] ; "parenthesized_roman")]
    #[test_case("Summary.\n\n\u{2022} item::\n\n  not code\n", &[false, false, false, false, false] ; "unicode_bullet")]
    #[test_case("Summary.\n\n.. [1] Note\n", &[false, false, false] ; "numeric_note_label_is_prose")]
    #[test_case("Summary.\n\n.. [#note] Note\n", &[false, false, false] ; "named_auto_note_label_is_prose")]
    #[test_case("Summary.\n\n.. [*] Note\n", &[false, false, false] ; "symbol_note_label_is_prose")]
    #[test_case("Summary.\n\n.. [citation label] text\n", &[false, false, true] ; "citation_label_with_spaces_is_a_comment")]
    #[test_case("Summary.\n\n.. |sub| text\n", &[false, false, true] ; "substitution_without_directive_is_not_prose")]
    #[test_case(".. _target: http://example.com\n", &[true] ; "target_is_not_prose")]
    #[test_case("__ http://example.com\n", &[true] ; "anonymous_target_is_not_prose")]
    #[test_case(".. |sub| replace:: text\n", &[true] ; "substitution_is_not_prose")]
    #[test_case("Summary.\n\n.. |sub| replace:: text\n\n    body\n", &[false, false, true, false, false] ; "substitution_content_is_prose")]
    #[test_case("Summary.\n\n.. |sub| code-block:: python\n\n    x = 1\n", &[false, false, true, true, true] ; "substitution_content_is_code")]
    #[test_case("Summary.\n\n.. code-block ::\n\n    x = 1\n", &[false, false, false, true, true] ; "directive_with_space_before_colons")]
    #[test_case("..\n\n    Block quote.\n", &[true, false, false] ; "empty_comment_leaves_the_block_after_it")]
    #[test_case("Summary.\n\n.. not a directive:: x\n\n    body\n", &[false, false, true, true, true] ; "comment_with_colons")]
    #[test_case("A paragraph\non more than\none line::\n    A literal block\n", &[false, false, false, true] ; "literal_block_after_several_lines")]
    #[test_case("Text\n    deeper\n", &[false, false] ; "definition_after_one_line")]
    #[test_case(".. a comment\n   with ``x\n\nText\n", &[true, true, true, false] ; "comment_is_not_prose")]
    #[test_case("Summary.\n\n.. raw:: html\n\n   <b>x</b>\n", &[false, false, false, true, true] ; "raw_content")]
    #[test_case("Summary.\n\nTitle\n==\n", &[false, false, false, false] ; "underline_shorter_than_four_is_text")]
    #[test_case("Summary.\n\n]\n```\nbody\n", &[false, false, false, true, false] ; "text_title_underlined_by_fence")]
    #[test_case("Summary.\n\n====\nTitle\n====\n\nText\n", &[false, false, true, false, true, false, false] ; "overline_title")]
    #[test_case("Summary.\n\n----\n\nText\n", &[false, false, true, false, false] ; "transition")]
    #[test_case("Summary.\n\nTitle\n    ====\n", &[false, false, false, false] ; "indented_underline_is_not_a_title")]
    #[test_case("Summary.\n\n====\nTitle\n", &[false, false, true, true] ; "incomplete_long_title")]
    #[test_case("Summary.\n\n==\nTitle\n", &[false, false, false, false] ; "incomplete_short_title_is_text")]
    #[test_case("Summary.\n\n====\nTitle\n----\n", &[false, false, true, true, true] ; "mismatched_overline_is_an_error")]
    #[test_case("Summary.\n\n#. item\nfoo\n", &[false, false, false, false] ; "auto_enumerator_without_next_item_is_text")]
    #[test_case("Summary.\n\na. item\nfoo\n", &[false, false, false, false] ; "lower_alpha_without_next_item_is_text")]
    #[test_case("Summary.\n\nA. item\nfoo\n", &[false, false, false, false] ; "upper_alpha_without_next_item_is_text")]
    #[test_case("Summary.\n\nz. item\nfoo\n", &[false, false, false, false] ; "alpha_past_z_is_text")]
    #[test_case("Summary.\n\nii. item\nfoo\n", &[false, false, false, false] ; "lower_roman_without_next_item_is_text")]
    #[test_case("Summary.\n\nIV. item\nfoo\n", &[false, false, false, false] ; "upper_roman_without_next_item_is_text")]
    #[test_case("Summary.\n\na. item\nb. next\n", &[false, false, false, false] ; "alpha_list_of_two")]
    #[test_case("Summary.\n\n#. item\n#. next\n", &[false, false, false, false] ; "auto_list_of_two")]
    #[test_case("Summary.\n\nab. item\nfoo\n", &[false, false, false, false] ; "two_letters_are_not_an_ordinal")]
    #[test_case("Summary.\n\nAb. item\nfoo\n", &[false, false, false, false] ; "mixed_case_is_not_an_ordinal")]
    #[test_case("Summary.\n\nZ. item\nfoo\n", &[false, false, false, false] ; "upper_alpha_z_is_an_ordinal_but_not_followed_by_one")]
    #[test_case("Summary.\n\n    --------\n\n    Paragraph.\n", &[false, false, true, false, false] ; "indented_transition")]
    #[test_case("\n    Summary.\n\n    }\n    ```\n\n    Also\n", &[false, false, false, false, true, false, false] ; "indented_title_is_measured_without_its_indentation")]
    #[test_case("Summary.\n\n|\n| Line block.\n|\n", &[false, false, false, false, false] ; "line_block_is_not_a_title")]
    fn lines_are_code_or_prose(text: &str, expected: &[bool]) {
        assert_eq!(classify_docstring(text).iter().map(|line| line.code).collect::<Vec<_>>(), expected);
    }

    #[test]
    fn classified_lines_start_at_their_offsets() {
        let starts: Vec<usize> =
            classify_docstring("Text.\n\n  More\n").iter().map(|line| usize::from(line.line.start)).collect();
        assert_eq!(starts, [0, 6, 7]);
    }

    #[test]
    fn five_thousand_nested_items_finish_in_linear_space() {
        let mut text = String::from("Summary.\n\n");
        for depth in 0..5000 {
            text.push_str(&" ".repeat(2 * depth));
            text.push_str("- x\n\n");
        }
        let started = Instant::now();
        let classified = classify_docstring(&text);
        assert!(started.elapsed() < Duration::from_secs(60));
        assert_eq!(classified.len(), 10_002);
        assert!(classified.iter().all(|line| !line.code));
    }
}
