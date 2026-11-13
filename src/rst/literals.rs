use super::blocks::Classified;
use super::lines::leading_whitespace;
use super::punctuation::{CLOSER_CHARS, CLOSING_DELIMITER_CHARS, DELIMITER_CHARS, OPENER_CHARS, match_chars};

// What is wrong with an inline literal or a backtick run, and so what the rule says.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Cause {
    // An inline literal that no later pair of backticks closes within its paragraph.
    Unclosed,
    // A literal whose closing pair is followed by a letter or digit, so the literal stays open, as in ``Foo``s.
    Suffix { literal: String, suffix: char },
    // A Markdown code fence, reported once at its opening line.
    Fence,
    // A run of one backtick closed by two, or of two closed by one.
    Mismatch { opened: usize, closed: usize },
}

// A problem at `at`, spanning `len` bytes of the docstring.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Finding {
    pub(crate) at: usize,
    pub(crate) len: usize,
    pub(crate) cause: Cause,
}

// The findings in a docstring's paragraphs, in the order they occur within each paragraph. A blank line, a code line
// or a line that opens a block ends a paragraph. A fence's own lines are reported as one fence, not as the literals in
// them.
pub(crate) fn literal_findings(lines: &[Classified]) -> Vec<Finding> {
    let mut found = Vec::new();
    let mut paragraph = Vec::new();
    let mut fences = Fences::default();
    for classified in lines {
        let line = classified.line;
        let at = usize::from(line.start);
        if line.text.is_empty() {
            fences.pending = None;
        } else if !classified.verbatim {
            fences.step(line.text, at);
        }
        if line.text.is_empty() || classified.code || classified.opens {
            found.extend(paragraph_findings(&paragraph));
            paragraph.clear();
        }
        if line.text.is_empty() || classified.code {
            continue;
        }
        if !paragraph.is_empty() {
            paragraph.push(('\n', at));
        }
        push_units(&mut paragraph, line.text, at);
    }
    found.extend(paragraph_findings(&paragraph));
    found.extend(fences.found);
    found
        .retain(|finding| finding.cause == Cause::Fence || !fences.spans.iter().any(|span| span.contains(&finding.at)));
    found
}

// The Markdown code fences of a docstring, as they are read line by line. A fence with an info string opens one, as in
// ```python. A bare fence opens one only when a closing fence follows it in the same paragraph, since a bare line of
// `~~~` is also a section underline. An opening fence is reported, and the lines of both fences are spans where the
// literals found are the fence's own.
#[derive(Default)]
struct Fences {
    found: Vec<Finding>,
    spans: Vec<std::ops::Range<usize>>,
    // The fence that is open, with its character and run length.
    open: Option<(char, usize)>,
    // A bare fence that a closing fence may still answer, with its character, run, offset and line span.
    pending: Option<(char, usize, usize, std::ops::Range<usize>)>,
}

impl Fences {
    fn step(&mut self, text: &str, at: usize) {
        let Some((char, run, rest)) = fence(text) else {
            return;
        };
        let line = at..at + text.len();
        if let Some((open_char, open_run)) = self.open {
            if rest.is_empty() && char == open_char && run >= open_run {
                self.open = None;
                self.spans.push(line);
            }
            return;
        }
        if let Some((pending_char, pending_run, pending_at, pending_line)) = self.pending.take() {
            if rest.is_empty() && char == pending_char && run >= pending_run {
                self.found.push(Finding { at: pending_at, len: pending_run, cause: Cause::Fence });
                self.spans.push(pending_line);
                self.spans.push(line);
            } else {
                self.pending = Some((pending_char, pending_run, pending_at, pending_line));
            }
            return;
        }
        let start = at + leading_whitespace(text).len();
        if rest.is_empty() {
            self.pending = Some((char, run, start, line));
        } else {
            self.found.push(Finding { at: start, len: run, cause: Cause::Fence });
            self.open = Some((char, run));
            self.spans.push(line);
        }
    }
}

// A Markdown code fence: three or more backticks or tildes, which a closing fence repeats alone on its line, and an
// opening fence may follow with an info string. The flag says whether the line has nothing after the run.
fn fence(text: &str) -> Option<(char, usize, &str)> {
    let trimmed = text.trim_start();
    let char = trimmed.chars().next().filter(|char| *char == '`' || *char == '~')?;
    let run = trimmed.chars().take_while(|other| *other == char).count();
    let rest = trimmed[run..].trim();
    (run >= 3 && !(char == '`' && rest.contains('`'))).then_some((char, run, rest))
}

// The findings of one paragraph: the literals docutils cannot close, and backtick runs that pair up wrongly. A literal
// that is a mismatch is reported as one, not also as unclosed.
fn paragraph_findings(units: &[(char, usize)]) -> Vec<Finding> {
    let mut found = open_literals(units);
    let mismatches = mismatches(units);
    found.retain(|finding| {
        finding.cause != Cause::Unclosed || !mismatches.iter().any(|mismatch| mismatch.at == finding.at)
    });
    found.extend(mismatches);
    found.sort_by_key(|finding| finding.at);
    found
}

// The characters of a paragraph with their offsets in the docstring. A backslash becomes a null that marks the next
// character as escaped, as docutils reads it.
fn push_units(units: &mut Vec<(char, usize)>, text: &str, at: usize) {
    let mut chars = text.char_indices();
    while let Some((index, char)) = chars.next() {
        if char == '\\' {
            units.push(('\0', at + index));
            if let Some((escaped_index, escaped)) = chars.next() {
                units.push((escaped, at + escaped_index));
            }
        } else {
            units.push((char, at + index));
        }
    }
}

// Scans a paragraph left to right as docutils does: a start-string is recognized only after whitespace or a
// punctuation mark, and a literal's end-string only after a non-space character, so text between backticks is read
// the way the rest of the paragraph is.
fn open_literals(units: &[(char, usize)]) -> Vec<Finding> {
    let mut found = Vec::new();
    // Where the text docutils still has to read begins, which a start-string at its beginning does not need a
    // character before.
    let mut segment = 0;
    let mut index = 0;
    while index < units.len() {
        if units[index].0 != '`' {
            index += 1;
            continue;
        }
        let Some(after) = start_at(units, index, segment) else {
            index += 1;
            continue;
        };
        // A start at the very beginning of the text is never quoted, so one that ends the paragraph still warns.
        let quoted =
            index != segment && units.get(after).is_none_or(|&(next, _)| match_chars(units[index - 1].0, next));
        if quoted {
            segment = after;
            index = after;
            continue;
        }
        if after == index + 2 {
            match literal_end(units, after) {
                Some(end) if end > after => {
                    segment = end + 2;
                    index = end + 2;
                }
                _ => {
                    let cause = suffix_cause(units, after).unwrap_or(Cause::Unclosed);
                    found.push(Finding { at: units[index].1, len: 2, cause });
                    segment = after;
                    index = after;
                }
            }
        } else {
            segment = interpreted_end(units, after).unwrap_or(after);
            index = segment;
        }
    }
    found
}

// The index after the start-string at `index`, when it can start markup: docutils wants whitespace or punctuation
// before it and a character other than a space after it.
fn start_at(units: &[(char, usize)], index: usize, segment: usize) -> Option<usize> {
    let double = units.get(index + 1).is_some_and(|&(next, _)| next == '`');
    let after = index + 1 + usize::from(double);
    let prefixed = index == segment || units[index - 1].0.is_whitespace() || starts_after(units[index - 1].0);
    let spaced = units.get(after).is_some_and(|&(next, _)| next.is_whitespace());
    (prefixed && !spaced).then_some(after)
}

// The index of the first pair of backticks that ends a literal from `from`, closing at its end.
fn literal_end(units: &[(char, usize)], from: usize) -> Option<usize> {
    (from..units.len().saturating_sub(1)).find(|&index| {
        units[index].0 == '`'
            && units[index + 1].0 == '`'
            && (index == from || !units[index - 1].0.is_whitespace())
            && ends_at(units, index + 2)
    })
}

// The index after the end of interpreted text that starts before `from`, closing at a backtick that follows a
// character other than whitespace or an escape.
fn interpreted_end(units: &[(char, usize)], from: usize) -> Option<usize> {
    (from..units.len()).find_map(|index| {
        let closing = units[index].0 == '`' && !units[index - 1].0.is_whitespace() && units[index - 1].0 != '\0';
        closing.then(|| suffix_end(units, index + 1)).flatten()
    })
}

// The index after the end-string's suffix: a reference marker of one or two underscores, when one follows, and then
// the end of the text or a character that may follow an end-string.
fn suffix_end(units: &[(char, usize)], index: usize) -> Option<usize> {
    let underscores = units.iter().skip(index).take(2).take_while(|&&(char, _)| char == '_').count();
    (0..=underscores).rev().map(|count| index + count).find(|&end| ends_at(units, end))
}

// The cause of a literal that no pair closes, when the first pair after it is followed by a letter or digit: the
// closing pair is then not an end-string, as in ``Foo``s.
fn suffix_cause(units: &[(char, usize)], from: usize) -> Option<Cause> {
    let close = (from..units.len().saturating_sub(1)).find(|&index| {
        units[index].0 == '`' && units[index + 1].0 == '`' && (index == from || !units[index - 1].0.is_whitespace())
    })?;
    let (suffix, _) = *units.get(close + 2)?;
    (close > from && suffix.is_alphanumeric()).then(|| Cause::Suffix {
        literal: units[from..close].iter().map(|(char, _)| char).filter(|char| **char != '\0').collect(),
        suffix,
    })
}

// The runs of backticks in a paragraph, as their start and length. An escaped backtick is a backtick here, since docutils
// pairs it with the next one when it closes a literal, as in ``a\``.
fn backtick_runs(units: &[(char, usize)]) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut index = 0;
    while index < units.len() {
        if units[index].0 == '`' {
            let start = index;
            while index < units.len() && units[index].0 == '`' {
                index += 1;
            }
            runs.push((start, index - start));
        } else {
            index += 1;
        }
    }
    runs
}

// Whether the backticks at `start` can open markup: whitespace or punctuation before them, and no space after.
fn opens_run(units: &[(char, usize)], start: usize, len: usize) -> bool {
    let prefixed = start == 0 || units[start - 1].0.is_whitespace() || starts_after(units[start - 1].0);
    prefixed && units.get(start + len).is_some_and(|&(next, _)| !next.is_whitespace())
}

// Whether the backticks at `start` can close markup: a character other than whitespace or an escape before them, and
// an end-string's suffix after them.
fn closes_run(units: &[(char, usize)], start: usize, len: usize) -> bool {
    start > 0
        && !units[start - 1].0.is_whitespace()
        && units[start - 1].0 != '\0'
        && suffix_end(units, start + len).is_some()
}

// The backtick runs that pair up wrongly within a paragraph, found by reading the runs in order: an opening run is
// closed by the next run that can close it. A run of one backtick closed by two is a mismatch, and so is a run of two
// closed by one.
//
// A literal may hold a single backtick, so a run of two closed by one is left alone only when no other run of two
// stands between it and the first pair of double backticks after it. A closing run followed by a role's colon is left
// to the role rules.
fn mismatches(units: &[(char, usize)]) -> Vec<Finding> {
    let runs = backtick_runs(units);
    let mut found = Vec::new();
    let mut open: Option<(usize, usize)> = None;
    for (position, &(start, len)) in runs.iter().enumerate() {
        if len > 2 {
            open = None;
            continue;
        }
        let Some((opened_at, opened)) = open else {
            if opens_run(units, start, len) {
                open = Some((start, len));
            }
            continue;
        };
        if !closes_run(units, start, len) {
            continue;
        }
        open = None;
        if len == opened {
            continue;
        }
        if opened == 2
            && let Some(first_pair) =
                runs[position + 1..].iter().position(|&(at, run)| run == 2 && closes_run(units, at, run))
            && !runs[position + 1..position + 1 + first_pair].iter().any(|&(_, run)| run == 2)
        {
            open = Some((opened_at, opened));
            continue;
        }
        if units.get(start + len).is_none_or(|&(next, _)| next != ':') {
            found.push(Finding { at: units[opened_at].1, len: opened, cause: Cause::Mismatch { opened, closed: len } });
        }
    }
    found
}

fn ends_at(units: &[(char, usize)], index: usize) -> bool {
    units.get(index).is_none_or(|&(char, _)| {
        char.is_whitespace()
            || char == '\0'
            || CLOSER_CHARS.contains(&char)
            || DELIMITER_CHARS.contains(&char)
            || CLOSING_DELIMITER_CHARS.contains(&char)
    })
}

// Whether a start-string may follow `char`, besides whitespace: an opening bracket or quote, or a delimiter.
fn starts_after(char: char) -> bool {
    OPENER_CHARS.contains(&char) || DELIMITER_CHARS.contains(&char)
}

#[cfg(test)]
mod tests {
    use test_case::test_case;

    use super::{Cause, Finding, literal_findings};
    use crate::rst::blocks::classify_docstring;

    #[test_case("Use ``x`` here.", &[] ; "closed")]
    #[test_case("Use ``x here.", &[4] ; "unclosed")]
    #[test_case("``a\nb`` c", &[] ; "closed_on_the_next_line")]
    #[test_case("``a`` and ``b", &[10] ; "second_unclosed")]
    #[test_case("Use ``x\n\nNext", &[4] ; "blank_line_ends_the_paragraph")]
    #[test_case("Text ``x\n.. code-block:: python\n    y\n", &[5] ; "code_line_ends_the_paragraph")]
    #[test_case("Use ``a and ``b`` here.", &[] ; "closing_pair_after_a_nested_start")]
    #[test_case("Use `` x`` here.", &[] ; "start_followed_by_space")]
    #[test_case("Use ``a `` here.", &[4] ; "end_preceded_by_space")]
    #[test_case("Use ``a``b c`` here.", &[] ; "end_followed_by_letter")]
    #[test_case("Use x``y`` here.", &[] ; "start_preceded_by_letter")]
    #[test_case("Use \\``x here.", &[] ; "escaped_start")]
    #[test_case("Use ```` here.", &[4] ; "four_backticks")]
    #[test_case("Python ``list``s are lists.", &[7] ; "end_followed_by_letter_is_unclosed")]
    #[test_case("Say \"``\" here.", &[] ; "quoted_start")]
    #[test_case("Use ``", &[] ; "start_at_the_end_of_text")]
    #[test_case("``", &[0] ; "start_that_is_the_whole_paragraph")]
    #[test_case("Summary.\n\n``\n", &[10] ; "start_on_its_own_line")]
    #[test_case("Summary.\n\n- Example::\n\n      code ``x\n", &[] ; "bullet_literal_body")]
    #[test_case("Summary.\n\n1. Example::\n\n       code ``x\n", &[] ; "enumerated_literal_body")]
    #[test_case("Summary.\n\n1. Use ``foo\nbar`` here.\n", &[] ; "enumerator_without_indented_next_line_is_text")]
    #[test_case("Summary.\n\ncivil. Use ``foo\nbar`` here.\n", &[] ; "civil_is_not_an_ordinal")]
    #[test_case("Summary.\n\nI. Use ``foo\nbar`` here.\n", &[] ; "roman_word_without_indented_next_line")]
    #[test_case("Summary.\n\nTitle ``x\n=====\n", &[16] ; "title_text_is_inline_markup")]
    #[test_case("Summary.\n\n]\n```\nbody ``x\n", &[21] ; "text_after_a_title_is_its_own_paragraph")]
    #[test_case("``a``", &[] ; "closed_at_the_end_of_text")]
    #[test_case("Use `x here.", &[] ; "interpreted_text_left_open")]
    #[test_case("Use `a`_ here.", &[] ; "interpreted_text_with_a_reference_end")]
    #[test_case("Use `a`__ here.", &[] ; "interpreted_text_with_an_anonymous_end")]
    #[test_case("Use `a`_x here.", &[] ; "reference_end_not_followed_by_space")]
    #[test_case("Use ``x\\", &[4] ; "backslash_at_the_end_of_the_line")]
    #[test_case(":literal:`a ``b` here.", &[] ; "double_backticks_inside_interpreted_text")]
    #[test_case(":param a: ``x\n:param b: y``\n", &[10] ; "literal_does_not_cross_fields")]
    #[test_case("- a ``x\n- b y``\n", &[4] ; "literal_does_not_cross_items")]
    fn literals_left_open(text: &str, expected: &[usize]) {
        let at: Vec<usize> = literal_findings(&classify_docstring(text)).iter().map(|finding| finding.at).collect();
        assert_eq!(at, expected);
    }

    fn describe(finding: &Finding) -> String {
        match &finding.cause {
            Cause::Unclosed => format!("{} unclosed", finding.at),
            Cause::Suffix { literal, suffix } => format!("{} suffix {literal} {suffix}", finding.at),
            Cause::Fence => format!("{} fence", finding.at),
            Cause::Mismatch { opened, closed } => format!("{} mismatch {opened} {closed}", finding.at),
        }
    }

    #[test_case("Summary.\n\nPython ``list``s are lists.\n", &["17 suffix list s"] ; "closing_pair_followed_by_a_letter")]
    #[test_case("``Foo``1 here\n", &["0 suffix Foo 1"] ; "closing_pair_followed_by_a_digit")]
    #[test_case("Summary.\n\n```rust\ncode\n```\n", &["10 fence"] ; "markdown_fence_is_reported_once")]
    #[test_case("Summary.\n\n~~~\ncode\n~~~\n", &["10 fence"] ; "tilde_fence")]
    #[test_case("Summary.\n\n~~~~~~~~\nTitle\n", &[] ; "bare_fence_without_a_closer_is_a_section_underline")]
    #[test_case("Summary.\n\n```python\ncode\n", &["10 fence"] ; "info_string_fence_without_a_closer")]
    #[test_case("Summary.\n\n```\n\ncode\n```\n", &["10 unclosed", "20 unclosed"] ; "closer_in_another_paragraph_does_not_pair")]
    #[test_case("Summary.\n\nCall `'AF_INET``, here.\n", &["15 mismatch 1 2"] ; "one_backtick_closed_by_two")]
    #[test_case("Summary.\n\nSee ``user_agent_appid` here.\n", &["14 mismatch 2 1"] ; "two_backticks_closed_by_one")]
    #[test_case("Summary.\n\nSee ``a` b`` here.\n", &[] ; "single_backtick_inside_a_literal_is_not_a_mismatch")]
    #[test_case("Summary.\n\nSee :func:`x`` here.\n", &["20 mismatch 1 2"] ; "role_target_closed_by_two")]
    #[test_case("Summary.\n\nSee ``xi` and ``argsi`` here.\n", &["14 mismatch 2 1"] ; "single_closes_a_run_that_swallows_markup")]
    #[test_case("Summary.\n\n``delete_file(filename)`.  If `data` is ``None``, then.\n", &["10 mismatch 2 1"] ; "single_closes_a_run_before_a_later_literal")]
    #[test_case("Summary.\n\nVerify ``HookCaller()`` and ``Y`` here.\n", &[] ; "closed_literals_are_not_mismatches")]
    #[test_case("Summary.\n\n~~~\n```\n~~~\n", &["14 unclosed", "10 fence"] ; "fence_pending_survives_a_different_fence_line")]
    #[test_case("Summary.\n\nSee `Link`_ and `Other`__ here.\n", &[] ; "reference_ends_are_not_mismatches")]
    #[test_case("Summary.\n\nSee `Link <https://x.y>`_ here.\n", &[] ; "reference_with_a_url_is_not_a_mismatch")]
    #[test_case("Summary.\n\nUse \\`a`` here.\n", &[] ; "escaped_backtick_is_not_a_run")]
    #[test_case(":literal:`a ``b` here.\n", &[] ; "double_backticks_inside_interpreted_text_are_not_a_mismatch")]
    fn findings_and_their_causes(text: &str, expected: &[&str]) {
        let described: Vec<String> = literal_findings(&classify_docstring(text)).iter().map(describe).collect();
        assert_eq!(described, expected);
    }
}
