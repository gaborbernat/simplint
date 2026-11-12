use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let Expr::Call(call) = expr else {
        return;
    };
    let Some(anchored) = anchored_call(checker, call) else {
        return;
    };
    let fix = fix(checker, call, &anchored);
    checker.report_fix(
        Rule::AnchoredRegexMatch,
        call.func.range(),
        if anchored.function == "search" {
            "`re.search` with a pattern anchored at both ends is `re.fullmatch`".to_owned()
        } else {
            format!("`re.{}` with a pattern anchored at its end is `re.fullmatch`", anchored.function)
        },
        fix,
        "Use `re.fullmatch`",
    );
}

pub(crate) struct Anchored<'a> {
    function: &'static str,
    pattern: &'a ast::ExprStringLiteral,
    // How many characters of the pattern's value the anchors take at its start and at its end.
    start: Anchor,
    end: Anchor,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Anchor {
    None,
    // `^`, `$`: `$` also matches before a newline that ends the string.
    Caret,
    Dollar,
    // `\A`, `\Z`: only at the very start and end.
    Absolute,
}

// A `re.match`, `re.prefixmatch` or `re.search` call with a literal pattern that only matches the whole string: anchored
// at its end, and for `re.search` at its start too, without flags, an inline flag or a top-level alternation, which
// would make an anchor apply to one branch only or to each line.
pub(crate) fn anchored_call<'a>(checker: &Checker, call: &'a ast::ExprCall) -> Option<Anchored<'a>> {
    let semantic = checker.semantic();
    let function = ["match", "prefixmatch", "search"]
        .into_iter()
        .find(|function| semantic.resolves_to(&call.func, &["re", function]))?;
    if call.arguments.len() != 2 || call.arguments.find_argument_value("string", 1).is_none() {
        return None;
    }
    let Expr::StringLiteral(pattern) = call.arguments.find_argument_value("pattern", 0)? else {
        return None;
    };
    if pattern.value.is_implicit_concatenated() {
        return None;
    }
    let value = pattern.value.to_str();
    let end = trailing_anchor(value)?;
    let start = leading_anchor(value);
    if function == "search" && start == Anchor::None {
        return None;
    }
    Some(Anchored { function, pattern, start, end })
}

fn leading_anchor(value: &str) -> Anchor {
    if value.starts_with('^') {
        Anchor::Caret
    } else if value.starts_with("\\A") {
        Anchor::Absolute
    } else {
        Anchor::None
    }
}

// The anchor at the end of a pattern with no flag group and no `|` outside a group or class.
fn trailing_anchor(value: &str) -> Option<Anchor> {
    let mut depth = 0_u32;
    let mut in_class = false;
    let mut escaped = false;
    let mut last = Anchor::None;
    let mut chars = value.chars().peekable();
    while let Some(char) = chars.next() {
        last = Anchor::None;
        if escaped {
            escaped = false;
            if char == 'Z' && !in_class {
                last = Anchor::Absolute;
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
                    if ahead.peek().is_some_and(|flag| "aiLmsux-".contains(*flag)) {
                        return None;
                    }
                }
                depth += 1;
            }
            ')' if !in_class => depth = depth.saturating_sub(1),
            '|' if !in_class && depth == 0 => return None,
            '$' if !in_class => last = Anchor::Dollar,
            _ => {}
        }
    }
    (last != Anchor::None && !in_class).then_some(last)
}

// Renaming the function and dropping the anchors keeps the match, except for a string that ends in a newline, which `$`
// lets through and `re.fullmatch` does not.
fn fix(checker: &Checker, call: &ast::ExprCall, anchored: &Anchored) -> Option<Fix> {
    let Expr::Attribute(attribute) = call.func.as_ref() else {
        return None;
    };
    let literal = checker.text(anchored.pattern);
    let quote_start = literal.find(['"', '\''])?;
    let prefix = &literal[..quote_start];
    let quote = if literal[quote_start..].starts_with("\"\"\"") || literal[quote_start..].starts_with("'''") {
        3
    } else {
        1
    };
    let body = TextRange::new(
        anchored.pattern.start() + TextSize::try_from(quote_start + quote).ok()?,
        anchored.pattern.end() - TextSize::try_from(quote).ok()?,
    );
    let source = checker.text(body);
    let raw = prefix.contains(['r', 'R']);
    let mut edits = vec![Edit::range_replacement("fullmatch".to_owned(), attribute.attr.range())];
    if anchored.start != Anchor::None {
        let width = anchor_width(source, anchored.start, raw, true)?;
        edits.push(Edit::range_deletion(TextRange::at(body.start(), width)));
    }
    let width = anchor_width(source, anchored.end, raw, false)?;
    edits.push(Edit::range_deletion(TextRange::new(body.end() - width, body.end())));
    let applicability = if anchored.end == Anchor::Dollar { Applicability::Unsafe } else { Applicability::Safe };
    let first = edits.remove(0);
    Some(Fix::applicable_edits(first, edits, applicability))
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
        (Anchor::None, _) => return None,
    };
    let found = candidates
        .iter()
        .find(|candidate| if leading { source.starts_with(**candidate) } else { source.ends_with(**candidate) })?;
    TextSize::try_from(found.len()).ok()
}
