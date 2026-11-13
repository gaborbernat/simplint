use std::collections::HashMap;
use std::ops::Range;

use super::blocks::Classified;
use super::directives::rst_walk;
use super::lines::Line;
use super::punctuation::{CLOSERS, OPENERS, ends_before, match_chars, starts_after};

// A `:role:target` with no backticks: where the role starts, where its target starts and ends, and whether backticks
// around the target make a valid reference.
pub(crate) struct BareRole {
    pub(crate) start: usize,
    pub(crate) target: usize,
    pub(crate) end: usize,
    pub(crate) fixable: bool,
}

// The target runs to the next whitespace, without the punctuation ending a sentence or a closing parenthesis the target
// did not open, so `(:meth:Reader.read).` ends at `read` while `:func:foo(x)` keeps its call.
pub(crate) fn bare_roles(lines: &[Classified]) -> Vec<BareRole> {
    // The roles of Sphinx and its Python domain that take a target in backticks, so a role written without them is
    // still recognized.
    const TARGET_ROLES: &[&str] = &[
        "class", "func", "meth", "attr", "mod", "obj", "exc", "data", "const", "ref", "doc", "term", "any", "code",
        "envvar", "file", "kbd", "abbr", "math",
    ];
    let mut found = Vec::new();
    for paragraph in rst_walk(lines).paragraphs {
        let (joined, offsets) = join(&paragraph);
        let masked = escape_mask(&joined);
        let plain = blank(&masked, &inline_spans(&masked));
        for (index, _) in plain.match_indices(':') {
            if index > 0 && !plain[..index].chars().next_back().is_some_and(starts_after) {
                continue;
            }
            let rest = &plain[index + 1..];
            // A role of the Python domain may name it, as `:py:func:`.
            let domain = if rest.starts_with("py:") { 3 } else { 0 };
            let Some(close) = rest[domain..].find(':') else {
                continue;
            };
            if !TARGET_ROLES.contains(&&rest[domain..domain + close]) {
                continue;
            }
            let after = &rest[domain + close + 1..];
            let mut target = &after[..after.find(char::is_whitespace).unwrap_or(after.len())];
            while let Some(last) = target.chars().next_back() {
                if ['.', ',', ';', ':', '!', '?', '\''].contains(&last) || last == '"' || unopened_closer(target) {
                    target = &target[..target.len() - last.len_utf8()];
                } else {
                    break;
                }
            }
            if !target.starts_with(|char: char| char.is_alphanumeric() || char == '_' || char == '~') {
                continue;
            }
            let target_start = index + 1 + domain + close + 1;
            found.push(BareRole {
                start: absolute(&offsets, index),
                target: absolute(&offsets, target_start),
                end: absolute(&offsets, target_start + target.len()),
                // A character a reference cannot hold, as the `=` of `:attr:debug=True`, leaves where the target ends
                // to the author.
                fixable: target.chars().all(|char| char.is_alphanumeric() || "_.~-/+()<>!".contains(char)),
            });
        }
    }
    found
}

// Whether the target ends with a closer that no opener in it pairs with, which the sentence or the brackets around the
// role closed.
fn unopened_closer(target: &str) -> bool {
    target.chars().last().is_some_and(|closer| {
        CLOSERS
            .chars()
            .position(|char| char == closer)
            .and_then(|at| OPENERS.chars().nth(at))
            .is_some_and(|opener| target.matches(closer).count() > target.matches(opener).count())
    })
}

// The target of each backticked role of Sphinx's Python domain in a paragraph, without the title of an explicit
// `Title <target>`.
pub(crate) fn references(paragraph: &[Line]) -> Vec<String> {
    let (joined, _) = join(paragraph);
    let masked = escape_mask(&joined);
    inline_spans(&masked)
        .into_iter()
        .filter_map(|span| match span.kind {
            Kind::Role { name, content } if object_role(name) => Some(role_target(&masked[content])),
            _ => None,
        })
        .collect()
}

// The MyST roles `{name}`content`` of a paragraph, whose content keeps its line breaks as spaces.
pub(crate) fn myst_references(paragraph: &[Line]) -> Vec<String> {
    let (joined, _) = join(paragraph);
    myst_roles(&joined)
        .into_iter()
        .filter(|(name, _)| object_role(name))
        .map(|(_, content)| role_target(&content.replace('\n', " ")))
        .collect()
}

// The length of the simple name at the start of a text, as `(?:(?!_)\w)+(?:[-._+:](?:(?!_)\w)+)*` matches it, with
// `char::is_alphanumeric` for `\w`, which also takes the combining marks that are alphabetic and Python's `\w` leaves out.
pub(crate) fn simplename_len(text: &str) -> usize {
    let mut end = 0;
    let mut position = 0;
    loop {
        let word: usize = text[position..].chars().take_while(|char| char.is_alphanumeric()).map(char::len_utf8).sum();
        if word == 0 {
            return end;
        }
        position += word;
        end = position;
        match text[position..].chars().next() {
            Some(separator) if "-._+:".contains(separator) => position += separator.len_utf8(),
            _ => return end,
        }
    }
}

// Inline markup of a paragraph as docutils reads it, where a backslash escapes the character after it. A kind of span
// is what its consumer skips: literal, target and role text is code, and emphasis and substitutions hold no roles.
enum Kind<'a> {
    Code,
    Markup,
    Role { name: &'a str, content: Range<usize> },
}

struct Span<'a> {
    range: Range<usize>,
    kind: Kind<'a>,
}

#[derive(Clone, Copy)]
enum Construct {
    Strong,
    Emphasis,
    Literal,
    Target,
    Substitution,
}

enum Candidate<'a> {
    Construct(Construct),
    Reference,
    Interpreted { role: Option<&'a str>, backtick: usize },
}

// The end of interpreted text: the backtick at `at` that closes it, the suffix role and reference mark after it, and
// where the text ends.
#[derive(Clone, Copy)]
struct Close<'a> {
    at: usize,
    role: Option<&'a str>,
    refend: bool,
    end: usize,
}

// The first end a search found from `from`, or that it found none. Every position between `from` and the end was
// rejected, so a search that starts at or after `from` and stops before the end finds the same one.
#[derive(Clone, Copy)]
struct Found<T> {
    from: usize,
    end: Option<T>,
}

// The searches of a paragraph's inline markup. Each start-string asks for the end of its construct, so the ends are
// memoized, and so is the simple name a word starts: a name from any position inside another name ends where it does.
struct Scanner<'a> {
    text: &'a str,
    name: (usize, usize),
    constructs: [Option<Found<(usize, usize)>>; 5],
    backticks: Option<Found<Close<'a>>>,
}

impl<'a> Scanner<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, name: (0, 0), constructs: [None; 5], backticks: None }
    }

    fn next_candidate(&mut self, base: usize) -> Option<(usize, Candidate<'a>, usize)> {
        let text = self.text;
        text[base..].char_indices().map(|(offset, _)| base + offset).find_map(|at| {
            if at > base && !text[..at].chars().next_back().is_some_and(starts_after) {
                return None;
            }
            let (candidate, matchend) = self.candidate_at(at)?;
            Some((at, candidate, matchend))
        })
    }

    fn candidate_at(&mut self, at: usize) -> Option<(Candidate<'a>, usize)> {
        let text = self.text;
        let tail = &text[at..];
        let following = |length: usize| text[at + length..].chars().next();
        if tail.starts_with("**") && non_space(following(2)) {
            return Some((Candidate::Construct(Construct::Strong), at + 2));
        }
        if tail.starts_with('*') && following(1) != Some('*') && non_space(following(1)) {
            return Some((Candidate::Construct(Construct::Emphasis), at + 1));
        }
        if tail.starts_with("``") && non_space(following(2)) {
            return Some((Candidate::Construct(Construct::Literal), at + 2));
        }
        if tail.starts_with("_`") && non_space(following(2)) {
            return Some((Candidate::Construct(Construct::Target), at + 2));
        }
        if tail.starts_with('|') && following(1) != Some('|') && non_space(following(1)) {
            return Some((Candidate::Construct(Construct::Substitution), at + 1));
        }
        let name = self.name_len(at);
        if name > 0 {
            let end = at + name;
            let underscores = if text[end..].starts_with("__") { 2 } else { 1 };
            if text[end..].starts_with('_') && text[end + underscores..].chars().next().is_none_or(ends_before) {
                return Some((Candidate::Reference, end + underscores));
            }
        }
        if let Some((role, length)) = self.role_at(at) {
            let backtick = at + length;
            if text[backtick..].starts_with('`')
                && !text[backtick..].starts_with("``")
                && non_space(following(length + 1))
            {
                return Some((Candidate::Interpreted { role: Some(role), backtick }, backtick + 1));
            }
            return None;
        }
        if tail.starts_with('`') && following(1) != Some('`') && non_space(following(1)) {
            return Some((Candidate::Interpreted { role: None, backtick: at }, at + 1));
        }
        None
    }

    // The length of the simple name at `at`, which is zero unless a word character starts there.
    fn name_len(&mut self, at: usize) -> usize {
        let text = self.text;
        if !text[at..].starts_with(char::is_alphanumeric) {
            return 0;
        }
        if !(self.name.0 <= at && at < self.name.1) {
            self.name = (at, at + simplename_len(&text[at..]));
        }
        self.name.1 - at
    }

    // A `:name:` role starting at `at`, with the length of the text it takes.
    fn role_at(&mut self, at: usize) -> Option<(&'a str, usize)> {
        let text = self.text;
        if !text[at..].starts_with(':') {
            return None;
        }
        let name_at = at + 1;
        let length = self.name_len(name_at);
        (length > 0 && text[name_at + length..].starts_with(':'))
            .then(|| (&text[name_at..name_at + length], length + 2))
    }

    // The end of a construct: the first end-string after `matchend` that follows a non-space, as the position it
    // starts at and its length. An empty content is no construct.
    fn construct_end(&mut self, matchend: usize, construct: Construct) -> Option<(usize, usize)> {
        let text = self.text;
        let slot = construct as usize;
        if let Some(found) = self.constructs[slot]
            && found.from <= matchend + 1
            && found.end.is_none_or(|(at, _)| at > matchend)
        {
            return found.end;
        }
        let end = text[matchend..].char_indices().skip(1).find_map(|(offset, _)| {
            let at = matchend + offset;
            construct_token(text, at, construct).map(|token| (at, token))
        });
        self.constructs[slot] = Some(Found { from: matchend + 1, end });
        end
    }

    // The end of interpreted text: the first backtick from `matchend` that closes it, as `interpreted_end` reads it.
    // A backtick right at `matchend` closes nothing, which ends the search.
    fn interpreted_end(&mut self, matchend: usize) -> Option<Close<'a>> {
        let found = match self.backticks {
            Some(found) if found.from <= matchend && found.end.is_none_or(|close| close.at >= matchend) => found.end,
            _ => {
                let end = self.backtick_close(matchend);
                self.backticks = Some(Found { from: matchend, end });
                end
            }
        };
        found.filter(|close| close.at > matchend)
    }

    fn backtick_close(&mut self, from: usize) -> Option<Close<'a>> {
        let text = self.text;
        for (offset, _) in text[from..].match_indices('`') {
            let at = from + offset;
            let prev = text[..at].chars().next_back();
            let before_prev = text[..at].chars().rev().nth(1);
            if prev.is_some_and(|char| char.is_whitespace() || char == '\0') && before_prev != Some('\0') {
                continue;
            }
            let suffix = self.role_at(at + 1).map(|(role, length)| (Some(role), length));
            for (role, role_length) in suffix.into_iter().chain([(None, 0)]) {
                for refend in [2, 1, 0] {
                    let marks = &text[at + 1 + role_length..];
                    if marks.starts_with(&"__"[..refend]) && marks[refend..].chars().next().is_none_or(ends_before) {
                        return Some(Close { at, role, refend: refend > 0, end: at + 1 + role_length + refend });
                    }
                }
            }
        }
        None
    }
}

fn inline_spans(text: &str) -> Vec<Span<'_>> {
    let mut scanner = Scanner::new(text);
    let mut spans = Vec::new();
    let mut base = 0;
    while let Some((start, candidate, matchend)) = scanner.next_candidate(base) {
        match candidate {
            Candidate::Reference => {
                spans.push(Span { range: start..matchend, kind: Kind::Code });
                base = matchend;
            }
            Candidate::Construct(construct) => {
                if quoted(text, base, start, matchend) {
                    base = matchend;
                    continue;
                }
                let (kind, end) = match scanner.construct_end(matchend, construct) {
                    Some((at, token)) => {
                        let kind = match construct {
                            Construct::Strong | Construct::Emphasis | Construct::Substitution => Kind::Markup,
                            Construct::Literal | Construct::Target => Kind::Code,
                        };
                        (kind, at + token)
                    }
                    None => (Kind::Code, matchend),
                };
                spans.push(Span { range: start..end, kind });
                base = end;
            }
            Candidate::Interpreted { role, backtick } => {
                let matchend = backtick + 1;
                if role.is_none() && quoted(text, base, start, matchend) {
                    base = matchend;
                    continue;
                }
                let Some(close) = scanner.interpreted_end(matchend) else {
                    spans.push(Span { range: start..matchend, kind: Kind::Code });
                    base = matchend;
                    continue;
                };
                let kind = match (role, close.role, close.refend) {
                    (None, Some(name), false) | (Some(name), None, false) => {
                        Kind::Role { name, content: matchend..close.at }
                    }
                    _ => Kind::Code,
                };
                spans.push(Span { range: start..close.end, kind });
                base = close.end;
            }
        }
    }
    spans
}

// Whether the token of a construct ends at `at`: the end-string follows a non-space and the position before it is
// allowed, which is the construct's own rule. It returns the end-string's length.
fn construct_token(text: &str, at: usize, construct: Construct) -> Option<usize> {
    let prev = text[..at].chars().next_back();
    let prev_ok = match construct {
        Construct::Literal => !prev.is_some_and(char::is_whitespace),
        _ => !prev.is_some_and(|char| char.is_whitespace() || char == '\0'),
    };
    if !prev_ok {
        return None;
    }
    let markers: &[&str] = match construct {
        Construct::Strong => &["**"],
        Construct::Literal => &["``"],
        Construct::Emphasis => &["*"],
        Construct::Target => &["`"],
        Construct::Substitution => &["|__", "|_", "|"],
    };
    let rest = &text[at..];
    markers
        .iter()
        .find(|marker| rest.starts_with(**marker) && rest[marker.len()..].chars().next().is_none_or(ends_before))
        .map(|marker| marker.len())
}

// Whether a start-string at `start` is quoted: an opener before it and its closer after it, or nothing after it.
fn quoted(text: &str, base: usize, start: usize, matchend: usize) -> bool {
    if start == base {
        return false;
    }
    match text[matchend..].chars().next() {
        None => true,
        Some(after) => text[..start].chars().next_back().is_some_and(|before| match_chars(before, after)),
    }
}

fn non_space(after: Option<char>) -> bool {
    after.is_none_or(|char| !char.is_whitespace())
}

// A paragraph's lines joined by line breaks, with where each line starts in the joined text and in the page.
fn join(paragraph: &[Line]) -> (String, Vec<(usize, usize)>) {
    let mut joined = String::new();
    let mut offsets = Vec::with_capacity(paragraph.len());
    for (index, line) in paragraph.iter().enumerate() {
        if index > 0 {
            joined.push('\n');
        }
        offsets.push((joined.len(), usize::from(line.start)));
        joined.push_str(line.text);
    }
    (joined, offsets)
}

fn absolute(offsets: &[(usize, usize)], index: usize) -> usize {
    let at = offsets.partition_point(|(start, _)| *start <= index) - 1;
    let (start, origin) = offsets[at];
    origin + index - start
}

// Each backslash becomes a NUL that the escaped character follows, so the escaped character starts and ends nothing
// while the text keeps its length.
fn escape_mask(text: &str) -> String {
    let mut masked = String::with_capacity(text.len());
    let mut chars = text.chars();
    while let Some(char) = chars.next() {
        if char == '\\' {
            masked.push('\0');
            masked.extend(chars.next());
        } else {
            masked.push(char);
        }
    }
    masked
}

// The text with the code of its inline markup replaced by spaces of the same length, which the bare roles of the prose
// are read from.
fn blank(masked: &str, spans: &[Span]) -> String {
    let mut plain = String::with_capacity(masked.len());
    let mut last = 0;
    for span in spans.iter().filter(|span| !matches!(span.kind, Kind::Markup)) {
        plain.push_str(&masked[last..span.range.start]);
        plain.push_str(&" ".repeat(span.range.len()));
        last = span.range.end;
    }
    plain.push_str(&masked[last..]);
    plain
}

fn object_role(name: &str) -> bool {
    const OBJECT_ROLES: [&str; 12] =
        ["func", "class", "meth", "attr", "data", "obj", "mod", "exc", "const", "type", "deco", "property"];
    let name = name.to_ascii_lowercase();
    OBJECT_ROLES.contains(&name.strip_prefix("py:").unwrap_or(&name))
}

// The target of a role's content: the text inside `<...>` of an explicit title, otherwise the content, with its escapes
// removed.
fn role_target(content: &str) -> String {
    unescape(explicit_title_target(content).unwrap_or(content))
}

// Sphinx's explicit title pattern, `^(.+?)\s*(?<!\x00)<(.*?)>$`: the first `<` after at least one character and not
// escaped, closed by the final `>`.
fn explicit_title_target(content: &str) -> Option<&str> {
    let inner = content.strip_suffix('>')?;
    let open = inner.char_indices().skip(1).find(|&(at, char)| char == '<' && !inner[..at].ends_with('\0'))?.0;
    Some(&inner[open + 1..])
}

fn unescape(text: &str) -> String {
    let mut unescaped = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\0' {
            if matches!(chars.peek(), Some(' ' | '\n')) {
                chars.next();
            }
        } else {
            unescaped.push(char);
        }
    }
    unescaped
}

// The roles of a MyST paragraph: `{name}` followed by a run of backticks, closed by the same run, as markdown-it's
// `myst_role` rule reads them, skipping code spans and escaped characters the way the inline rules do.
fn myst_roles(text: &str) -> Vec<(&str, &str)> {
    let runs = BacktickRuns::new(text);
    let closers: Vec<usize> = text.match_indices('}').map(|(at, _)| at).collect();
    let mut roles = Vec::new();
    let mut position = 0;
    while let Some(offset) = text[position..].find(['`', '\\', '{', '<', '[', ']']) {
        let at = position + offset;
        let tail = &text[at..];
        if let Some(escaped) = tail.strip_prefix('\\') {
            let escaped = escaped.chars().next().filter(char::is_ascii_punctuation);
            position = at + 1 + escaped.map_or(0, char::len_utf8);
        } else if tail.starts_with('`') {
            let run = count_backticks(tail);
            position = match runs.closing(at + run, run) {
                Some(closing) => closing + run,
                None => at + run,
            };
        } else if let Some((name, content, end)) = myst_role_at(text, at, &closers, &runs) {
            roles.push((name, content));
            position = end;
        } else {
            position = at + raw_span(text, at).unwrap_or(1);
        }
    }
    roles
}

// The length of the markup at `at` that holds no roles: an autolink or inline HTML, a reference definition's line, or
// the destination of a link.
fn raw_span(text: &str, at: usize) -> Option<usize> {
    let tail = &text[at..];
    if tail.starts_with('<') {
        raw_markup_length(tail)
    } else if tail.starts_with('[') {
        reference_definition_length(text, at)
    } else if tail.starts_with("](") {
        tail.find(')').map(|close| close + 1)
    } else {
        None
    }
}

// An autolink, `<scheme:...>`, or an opening, closing or empty HTML tag, `<name ...>`, which ends at the first `>`.
fn raw_markup_length(tail: &str) -> Option<usize> {
    let close = tail.find('>')?;
    let inner = &tail[1..close];
    let autolink = inner.split_once(':').is_some_and(|(scheme, _)| {
        scheme.starts_with(|char: char| char.is_ascii_alphabetic())
            && scheme.chars().all(|char| char.is_ascii_alphanumeric() || "+.-".contains(char))
    }) && !inner.contains(|char: char| char == '<' || char.is_whitespace());
    let html =
        inner.starts_with(|char: char| char.is_ascii_alphabetic() || "/!?".contains(char)) && !inner.contains('<');
    (autolink || html).then_some(close + 1)
}

// A reference definition, `[label]: destination`, which is a line of its own that holds no roles.
fn reference_definition_length(text: &str, at: usize) -> Option<usize> {
    let starts_line = text[..at].chars().next_back().is_none_or(|char| char == '\n');
    let line_end = text[at..].find('\n').map_or(text.len(), |offset| at + offset);
    (starts_line && text[at..line_end].contains("]:")).then_some(line_end - at)
}

fn count_backticks(text: &str) -> usize {
    text.chars().take_while(|&char| char == '`').count()
}

// The maximal runs of backticks of a text, by their length, so the run that closes a code span is found by a search
// rather than a scan from each opening run.
struct BacktickRuns(HashMap<usize, Vec<usize>>);

impl BacktickRuns {
    fn new(text: &str) -> Self {
        let mut by_length: HashMap<usize, Vec<usize>> = HashMap::new();
        let mut at = 0;
        while let Some(offset) = text[at..].find('`') {
            let start = at + offset;
            let length = count_backticks(&text[start..]);
            by_length.entry(length).or_default().push(start);
            at = start + length;
        }
        Self(by_length)
    }

    // The first run of exactly `length` backticks that starts at or after `from`.
    fn closing(&self, from: usize, length: usize) -> Option<usize> {
        let starts = self.0.get(&length)?;
        starts.get(starts.partition_point(|&start| start < from)).copied()
    }
}

// A `{name}` role whose backtick run starts right after the closing brace, with the position of the first closing brace
// after `at` taken from `closers`.
fn myst_role_at<'a>(
    text: &'a str,
    at: usize,
    closers: &[usize],
    runs: &BacktickRuns,
) -> Option<(&'a str, &'a str, usize)> {
    let close = *closers.get(closers.partition_point(|&closer| closer <= at))?;
    let name = &text[at + 1..close];
    let valid = !name.is_empty() && name.chars().all(|char| char.is_ascii_alphanumeric() || "_-+:".contains(char));
    if !valid || text[..at].ends_with('\\') {
        return None;
    }
    let ticks_at = close + 1;
    let run = count_backticks(&text[ticks_at..]);
    if run == 0 {
        return None;
    }
    let content_at = ticks_at + run;
    let body = content_at + text[content_at..].chars().next()?.len_utf8();
    let closing = runs.closing(body, run)?;
    Some((name, &text[content_at..closing], closing + run))
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{bare_roles, myst_references, references};
    use crate::rst::blocks::classify_docstring;
    use crate::rst::lines::lines;
    use crate::rst::{page_lines, rst_markup};

    fn targets(text: &str) -> Vec<String> {
        references(&lines(text))
    }

    #[test_case(":func:`~pkg.mod.name`.", &["~pkg.mod.name"] ; "tilde_prefix_is_left_to_the_caller")]
    #[test_case(":class:`Title <pkg.Thing>`", &["pkg.Thing"] ; "explicit_title")]
    #[test_case(":class:`Title <~pkg.Thing>`", &["~pkg.Thing"] ; "explicit_title_keeps_its_tilde")]
    #[test_case(":py:meth:`x`", &["x"] ; "python_domain_prefix")]
    #[test_case(":func:`a` and :class:`b`", &["a", "b"] ; "two_roles")]
    #[test_case(":ref:`label`", &[] ; "role_outside_the_object_roles")]
    #[test_case(":FUNC:`x`", &["x"] ; "role_name_ignores_case")]
    #[test_case("`x`:func:", &["x"] ; "suffix_role_takes_the_target")]
    #[test_case("`pkg.name`:func:", &["pkg.name"] ; "suffix_role_name")]
    #[test_case(":func:`x`_", &[] ; "role_with_a_reference_mark_is_a_phrase_reference")]
    #[test_case(":c:func:`name`", &[] ; "c_domain_role_is_not_python")]
    #[test_case("x:func:`y`", &[] ; "role_glued_to_a_word")]
    #[test_case("*:func:`x`*", &[] ; "emphasis_holds_no_roles")]
    #[test_case("|:func:`x`|", &[] ; "substitution_holds_no_roles")]
    #[test_case("``:func:`x` ``", &["x"] ; "role_after_an_unclosed_literal_start")]
    #[test_case("Use ``a`b`` and :func:`c` here.", &["c"] ; "single_backtick_inside_a_literal")]
    #[test_case(":func:`a\\`b`", &["a`b"] ; "escaped_backtick_in_a_target")]
    #[test_case(":func:`title \\<x>`", &["title <x>"] ; "escaped_angle_is_not_a_title")]
    #[test_case(":func:``x``", &[] ; "double_backtick_is_a_literal")]
    #[test_case(":func:`a\nb`", &["a\nb"] ; "target_across_lines")]
    #[test_case(":class:`the thing\n<pkg.Thing>`", &["pkg.Thing"] ; "explicit_title_across_lines")]
    #[test_case(":func:`\\ `", &[""] ; "escaped_space_is_an_empty_target")]
    #[test_case("`\\ `:func:", &[""] ; "escaped_space_suffix_target_is_empty")]
    #[test_case("name_ and :func:`x`", &["x"] ; "reference_mark_before_a_role")]
    #[test_case("name__ and :func:`x`", &["x"] ; "anonymous_reference_mark_before_a_role")]
    #[test_case("**:func:`x`** and :func:`y`", &["y"] ; "strong_emphasis_holds_no_roles")]
    #[test_case("_`a` and :func:`y`", &["y"] ; "inline_target_holds_no_roles")]
    #[test_case("`unclosed :func:`x", &[] ; "unclosed_interpreted_text")]
    #[test_case("(`) :func:`x`", &["x"] ; "backtick_quoted_by_parentheses")]
    #[test_case("'`' :func:`x`", &["x"] ; "backtick_quoted_by_quotes")]
    #[test_case("<`> :func:`x`", &["x"] ; "backtick_quoted_by_angle_brackets")]
    #[test_case("[`] :func:`x`", &["x"] ; "backtick_quoted_by_brackets")]
    #[test_case("{`} :func:`x`", &["x"] ; "backtick_quoted_by_braces")]
    #[test_case("「:func:`x`」", &["x"] ; "cjk_corner_brackets_open_and_close")]
    #[test_case(":func:`x`）", &["x"] ; "fullwidth_right_parenthesis_closes_a_role")]
    #[test_case("«:func:`x`»", &["x"] ; "guillemets_open_and_close_a_role")]
    #[test_case("‘:func:`x`’", &["x"] ; "single_curly_quotes_open_and_close_a_role")]
    #[test_case("‘`‚ :func:`x`", &["x"] ; "quote_pair_of_non_ascii_quotes_is_quoted")]
    #[test_case("Use\u{2014}:func:`pkg.a` here.", &["pkg.a"] ; "em_dash_before_a_role")]
    #[test_case(":func:`pkg.a`\u{2013}here.", &["pkg.a"] ; "en_dash_after_a_role")]
    #[test_case(":func:`pkg.a`\u{2026} here.", &["pkg.a"] ; "ellipsis_after_a_role")]
    #[test_case("\u{4f7f}\u{7528} :func:`pkg.a`\u{3002}", &["pkg.a"] ; "ideographic_full_stop_after_a_role")]
    #[test_case("x\u{ff0e}:func:`pkg.a` y", &["pkg.a"] ; "fullwidth_full_stop_before_a_role")]
    #[test_case("x \\\\:func:`pkg.a` y", &[] ; "escaped_backslash_before_a_role")]
    #[test_case(":func:`pkg.a`\\\\ y", &["pkg.a"] ; "escaped_backslash_after_a_role")]
    #[test_case("x\\:func:`pkg.a` y", &[] ; "escaped_colon_before_a_role")]
    fn backticked_targets(line: &str, expected: &[&str]) {
        assert_eq!(targets(line), expected);
    }

    #[test_case("See :meth:Reader.read.", &[(4, 10, 21, true)] ; "trailing_full_stop")]
    #[test_case("(:meth:Reader.read).", &[(1, 7, 18, true)] ; "closing_parenthesis_it_did_not_open")]
    #[test_case(":func:foo(x)", &[(0, 6, 12, true)] ; "call_keeps_its_parentheses")]
    #[test_case(":attr:debug=True", &[(0, 6, 16, false)] ; "character_a_reference_cannot_hold")]
    #[test_case(":py:func:foo", &[(0, 9, 12, true)] ; "python_domain_prefix_bare")]
    #[test_case("a:func:foo", &[] ; "role_inside_a_word")]
    #[test_case(":guilabel:foo", &[] ; "role_outside_the_target_roles")]
    #[test_case(":func:-x", &[] ; "target_not_starting_with_a_name")]
    #[test_case("[:func:foo]", &[(1, 7, 10, true)] ; "bare_role_inside_brackets")]
    #[test_case("‘:func:foo", &[(3, 9, 12, true)] ; "bare_role_after_a_non_ascii_opener")]
    #[test_case("（:class:Path）", &[(3, 10, 14, true)] ; "bare_role_inside_fullwidth_parentheses")]
    #[test_case("「:class:Path」", &[(3, 10, 14, true)] ; "bare_role_inside_corner_brackets")]
    #[test_case("\":func:foo\"", &[(1, 7, 10, true)] ; "bare_role_after_a_quote")]
    #[test_case("``x :func:foo`` here", &[] ; "bare_role_in_an_inline_literal")]
    #[test_case("`x`:func:foo", &[(3, 9, 12, true)] ; "bare_role_after_interpreted_text")]
    #[test_case("a\\:func:foo", &[] ; "bare_role_after_an_escaped_colon")]
    fn bare_roles_and_their_target_offsets(text: &str, expected: &[(usize, usize, usize, bool)]) {
        let found: Vec<_> = bare_roles(&classify_docstring(text))
            .iter()
            .map(|role| (role.start, role.target, role.end, role.fixable))
            .collect();
        assert_eq!(found, expected);
    }

    fn timed<T>(work: impl FnOnce() -> T) -> T {
        let started = std::time::Instant::now();
        let result = work();
        assert!(started.elapsed() < std::time::Duration::from_secs(5), "took {:?}", started.elapsed());
        result
    }

    #[test]
    fn grid_rows_read_in_linear_time() {
        let border = format!("+{}+\n", "-".repeat(15));
        let text = format!("{border}{}{border}", "| :func:`pkg.a` |\n".repeat(10_000));
        assert_eq!(timed(|| rst_markup(&page_lines(&text)).references).len(), 10_000);
    }

    #[test]
    fn paragraph_of_calls_reads_in_linear_time() {
        let text = format!("{}:func:`pkg.a`", "Call f(*args) here.\n".repeat(10_000));
        assert_eq!(timed(|| references(&lines(&text))), ["pkg.a"]);
    }

    #[test]
    fn long_line_of_unclosed_literals_reads_in_linear_time() {
        let text = format!("{}:func:`pkg.a`", "``a ".repeat(100_000));
        assert_eq!(timed(|| references(&lines(&text))), ["pkg.a"]);
    }

    #[test]
    fn long_name_chain_reads_in_linear_time() {
        let text = format!("{} :func:`pkg.a`", "a:".repeat(100_000));
        assert_eq!(timed(|| references(&lines(&text))), ["pkg.a"]);
    }

    #[test]
    fn bare_role_in_a_code_block_is_not_reported() {
        assert!(bare_roles(&classify_docstring("Example::\n\n    :func:foo\nText\n")).is_empty());
    }

    #[test]
    fn bare_role_in_a_comment_is_not_reported() {
        assert!(bare_roles(&classify_docstring(".. :func:foo\n")).is_empty());
    }

    #[test_case("{func}`xyz`", &["xyz"] ; "myst_role")]
    #[test_case("{func}``xyz``", &["xyz"] ; "myst_role_with_two_ticks")]
    #[test_case("{func}``x`yz``", &["x`yz"] ; "myst_role_with_a_tick_inside")]
    #[test_case("{func}``xyz`", &[] ; "myst_role_with_unbalanced_ticks")]
    #[test_case("{func}`` a", &[] ; "myst_role_with_empty_content")]
    #[test_case("\\{func}`xyz`", &[] ; "escaped_myst_role")]
    #[test_case("`` {func}`xyz` ``", &[] ; "myst_role_in_a_code_span")]
    #[test_case(":func:`xyz`", &[] ; "rst_role_in_markdown")]
    #[test_case("{func}`xy\nz` `d\ne`", &["xy z"] ; "myst_role_content_across_lines")]
    #[test_case("{fu nc}`xyz`", &[] ; "myst_role_name_with_a_space")]
    #[test_case("{func`xyz`", &[] ; "myst_role_name_without_a_closing_brace")]
    #[test_case("{func}`", &[] ; "myst_role_ticks_at_the_end_of_the_text")]
    #[test_case("\\\\{func}`xyz`", &[] ; "myst_role_after_an_escaped_backslash")]
    #[test_case("`a` and {func}`xyz`", &["xyz"] ; "myst_role_after_a_code_span")]
    fn myst_roles_in_a_paragraph(text: &str, expected: &[&str]) {
        assert_eq!(myst_references(&lines(text)), expected);
    }
}
