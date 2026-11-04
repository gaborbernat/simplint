use crate::lint::checker::Checker;
use crate::lint::noqa::{after_tool, is_code};
use crate::lint::registry::Rule;

pub fn suppression_without_reason(checker: &Checker) {
    for range in checker.comment_ranges() {
        let segments: Vec<Segment> = checker.text(range).split('#').map(Segment::parse).collect();
        let directives: Vec<&Segment> = segments.iter().filter(|segment| segment.directive.is_some()).collect();
        if directives.is_empty() {
            continue;
        }
        if let Some(blanket) = directives.iter().find(|segment| segment.missing_codes) {
            let message = format!(
                "`{}` names no error code; list the codes it suppresses",
                blanket.directive.unwrap_or_default()
            );
            checker.report(Rule::SuppressionWithoutReason, range, message);
        } else if !segments.iter().any(|segment| segment.has_reason) {
            checker.report(
                Rule::SuppressionWithoutReason,
                range,
                "Suppression comment gives no reason; add one after the codes",
            );
        }
    }
}

// The directives as `tool: action`, read the way the suppressions themselves are, or a bare action, and how each names
// its codes.
const DIRECTIVES: [(Option<&str>, &str, Codes); 12] = [
    (None, "noqa", Codes::Colon),
    (Some("ruff"), "noqa", Codes::Colon),
    (Some("flake8"), "noqa", Codes::Colon),
    (Some("ruff"), "ignore", Codes::Bracket),
    (Some("ruff"), "file-ignore", Codes::Bracket),
    (Some("simplint"), "ignore", Codes::Bracket),
    (Some("type"), "ignore", Codes::Bracket),
    (Some("ty"), "ignore", Codes::Bracket),
    (Some("pyright"), "ignore", Codes::Bracket),
    (Some("pyrefly"), "ignore", Codes::Bracket),
    (Some("pragma"), "no cover", Codes::None),
    (Some("pragma"), "no branch", Codes::None),
];

struct Segment<'a> {
    // The directive as the comment spells it.
    directive: Option<&'a str>,
    missing_codes: bool,
    has_reason: bool,
}

impl<'a> Segment<'a> {
    fn parse(segment: &'a str) -> Self {
        let text = segment.trim();
        let lower = text.to_ascii_lowercase();
        for (tool, action, codes) in DIRECTIVES {
            let after_tool = tool.map_or(Some(lower.as_str()), |tool| after_tool(&lower, tool));
            if let Some(rest) = after_tool.and_then(|after_tool| after_tool.strip_prefix(action))
                && !rest.starts_with(|char: char| char.is_alphanumeric() || matches!(char, '-' | '_'))
            {
                let (directive, rest) = text.split_at(text.len() - rest.len());
                let (named, reason) = codes.split(rest);
                let missing_codes = !named && !matches!(codes, Codes::None);
                return Self { directive: Some(directive), missing_codes, has_reason: !reason.trim().is_empty() };
            }
        }
        Self { directive: None, missing_codes: false, has_reason: !text.is_empty() }
    }
}

#[derive(Clone, Copy)]
enum Codes {
    Colon,
    Bracket,
    None,
}

impl Codes {
    fn split(self, rest: &str) -> (bool, &str) {
        match self {
            Self::Colon => {
                let Some(codes) = rest.trim_start().strip_prefix(':') else {
                    return (false, rest);
                };
                let mut rest = codes;
                let mut named = false;
                loop {
                    let trimmed = rest.trim_start_matches(|char: char| char == ',' || char.is_whitespace());
                    let end = trimmed.find(|char: char| char == ',' || char.is_whitespace()).unwrap_or(trimmed.len());
                    if end == 0 || !is_code(&trimmed[..end]) {
                        return (named, trimmed);
                    }
                    named = true;
                    rest = &trimmed[end..];
                }
            }
            Self::Bracket => match rest.strip_prefix('[').and_then(|inner| inner.split_once(']')) {
                Some((codes, reason)) => (!codes.trim().is_empty(), reason),
                None => (false, rest),
            },
            Self::None => (true, rest),
        }
    }
}
