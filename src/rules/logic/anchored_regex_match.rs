use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::edits::fix_of;
use crate::rules::logic::regex_pattern::INLINE_FLAGS;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::Call(call) = expr else {
        return;
    };
    let checker = walker.checker;
    let Some(anchored) = anchored_call(checker, call) else {
        return;
    };
    checker.report_fix(
        Rule::AnchoredRegexMatch,
        call.func.range(),
        if anchored.function == "search" {
            "`re.search` with a pattern anchored at both ends is `re.fullmatch`".to_owned()
        } else {
            format!("`re.{}` with a pattern anchored at its end is `re.fullmatch`", anchored.function)
        },
        fix(checker, call, &anchored),
        "Use `re.fullmatch`",
    );
}

pub(crate) struct Anchored<'a> {
    function: &'static str,
    pattern: &'a ast::StringLiteral,
    start: Option<Anchor>,
    end: Anchor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Anchor {
    // `^`, `$`: `$` also matches before a newline that ends the string.
    Caret,
    Dollar,
    // `\A`, `\Z`: only at the very start and end.
    Absolute,
}

// A `re.match`, `re.prefixmatch` or `re.search` call with a literal pattern that only matches the whole string:
// anchored at its end, and for `re.search` at its start too, without flags, an inline flag or a top-level alternation,
// which would make an anchor apply to one branch only or to each line.
pub(crate) fn anchored_call<'a>(checker: &Checker, call: &'a ast::ExprCall) -> Option<Anchored<'a>> {
    if call.arguments.len() != 2 || call.arguments.find_argument_value("string", 1).is_none() {
        return None;
    }
    let semantic = checker.semantic();
    let function = ["match", "prefixmatch", "search"]
        .into_iter()
        .find(|function| semantic.resolves_to(&call.func, &["re", function]))?;
    let pattern =
        call.arguments.find_argument_value("pattern", 0)?.as_string_literal_expr()?.as_single_part_string()?;
    let value = pattern.value.as_ref();
    let end = trailing_anchor(value)?;
    let start = leading_anchor(value);
    if function == "search" && start.is_none() {
        return None;
    }
    Some(Anchored { function, pattern, start, end })
}

// The anchor at the end of a pattern with no flag group and no `|` outside a group or class.
fn trailing_anchor(value: &str) -> Option<Anchor> {
    let mut depth = 0_u32;
    let mut in_class = false;
    let mut escaped = false;
    let mut last = None;
    let mut chars = value.chars().peekable();
    while let Some(char) = chars.next() {
        last = None;
        if escaped {
            escaped = false;
            if char == 'Z' && !in_class {
                last = Some(Anchor::Absolute);
            }
            continue;
        }
        match char {
            '\\' => escaped = true,
            '[' if !in_class => in_class = true,
            ']' if in_class => in_class = false,
            '(' if !in_class => {
                if chars.peek() == Some(&'?') {
                    let mut ahead = chars.clone();
                    ahead.next();
                    if ahead
                        .peek()
                        .is_some_and(|&flag| u8::try_from(flag).is_ok_and(|flag| INLINE_FLAGS.contains(&flag)))
                    {
                        return None;
                    }
                }
                depth += 1;
            }
            ')' if !in_class => depth = depth.saturating_sub(1),
            '|' if !in_class && depth == 0 => return None,
            '$' if !in_class => last = Some(Anchor::Dollar),
            _ => {}
        }
    }
    last.filter(|_| !in_class)
}

fn leading_anchor(value: &str) -> Option<Anchor> {
    if value.starts_with('^') {
        Some(Anchor::Caret)
    } else if value.starts_with("\\A") {
        Some(Anchor::Absolute)
    } else {
        None
    }
}

// Renaming the function and dropping the anchors keeps the match, except for a string that ends in a newline, which `$`
// lets through and `re.fullmatch` does not.
fn fix(checker: &Checker, call: &ast::ExprCall, anchored: &Anchored) -> Option<Fix> {
    let Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    let body = anchored.pattern.content_range();
    let source = checker.text(body);
    let raw = anchored.pattern.flags.prefix().is_raw();
    let mut edits = vec![Edit::range_replacement("fullmatch".to_owned(), attribute.attr.range())];
    if let Some(start) = anchored.start {
        edits.push(Edit::range_deletion(TextRange::at(body.start(), anchor_width(source, start, raw, true)?)));
    }
    let width = anchor_width(source, anchored.end, raw, false)?;
    edits.push(Edit::range_deletion(TextRange::new(body.end() - width, body.end())));
    fix_of(edits, if anchored.end == Anchor::Dollar { Applicability::Unsafe } else { Applicability::Safe })
}

// The source characters of an anchor: one for `^` and `$`, two for `\A` and `\Z`, three when a plain string writes
// the backslash as `\\`.
fn anchor_width(source: &str, anchor: Anchor, raw: bool, leading: bool) -> Option<TextSize> {
    let candidates: &[&str] = match (anchor, leading) {
        (Anchor::Caret, _) => &["^"],
        (Anchor::Dollar, _) => &["$"],
        (Anchor::Absolute, true) if raw => &["\\A"],
        (Anchor::Absolute, true) => &["\\\\A", "\\A"],
        (Anchor::Absolute, false) if raw => &["\\Z"],
        (Anchor::Absolute, false) => &["\\\\Z", "\\Z"],
    };
    let found = candidates
        .iter()
        .find(|candidate| if leading { source.starts_with(**candidate) } else { source.ends_with(**candidate) })?;
    Some(TextSize::of(*found))
}
