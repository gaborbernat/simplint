use crate::lint::noqa::{after_tool, is_code};

// The suppression directives SMP003 and SMP004 read, as the table spells them, how each names its codes and whether it
// covers one line or the whole file.
const DIRECTIVES: [(&str, Codes, Scope); 12] = [
    ("noqa", Codes::Colon, Scope::Line),
    ("ruff: noqa", Codes::Colon, Scope::File),
    ("flake8: noqa", Codes::Colon, Scope::File),
    ("ruff: ignore", Codes::Bracket, Scope::Line),
    ("ruff: file-ignore", Codes::Bracket, Scope::File),
    ("simplint: ignore", Codes::Bracket, Scope::Line),
    ("type: ignore", Codes::Bracket, Scope::Line),
    ("ty: ignore", Codes::Bracket, Scope::Line),
    ("pyright: ignore", Codes::Bracket, Scope::Line),
    ("pyrefly: ignore", Codes::Bracket, Scope::Line),
    ("pragma: no cover", Codes::None, Scope::Line),
    ("pragma: no branch", Codes::None, Scope::Line),
];

// The directives of one comment, and whether the comment says anything beyond them.
pub(crate) struct Comment {
    pub(super) directives: Vec<Directive>,
    pub(super) has_reason: bool,
}

#[derive(Clone, Copy)]
pub(super) struct Directive {
    // The directive as the table spells it, such as `ty: ignore`.
    pub(super) name: &'static str,
    pub(super) codes: Codes,
    pub(super) scope: Scope,
    pub(super) missing_codes: bool,
}

#[derive(Clone, Copy)]
pub(super) enum Codes {
    Colon,
    Bracket,
    None,
}

#[derive(Clone, Copy)]
pub(super) enum Scope {
    Line,
    File,
}

impl Comment {
    pub(crate) fn parse(comment: &str) -> Self {
        let mut directives = Vec::new();
        let mut has_reason = false;
        for segment in comment.split('#') {
            let text = segment.trim();
            if !may_be_directive(text) {
                has_reason |= !text.is_empty();
                continue;
            }
            match parse_directive(text) {
                Some((directive, reason)) => {
                    has_reason |= !reason.trim().is_empty();
                    directives.push(directive);
                }
                None => has_reason |= !text.is_empty(),
            }
        }
        Self { directives, has_reason }
    }
}

impl Directive {
    // The directive as a user would write it with codes, for the message that asks for them. Only directives that take
    // codes are reported, so `Codes::None` never reaches this.
    pub(super) fn example(&self) -> String {
        match self.codes {
            Codes::Colon => format!("{}: CODE", self.name),
            Codes::Bracket | Codes::None => format!("{}[code]", self.name),
        }
    }
}

// Every directive names a tool followed by a colon, or is `noqa`, so a segment with neither cannot be one. Checking
// that first keeps the lowercasing of `parse_directive` off the segments most comments are made of.
fn may_be_directive(text: &str) -> bool {
    text.contains(':') || text.as_bytes().windows(4).any(|window| window.eq_ignore_ascii_case(b"noqa"))
}

// The directive at the start of `text`, and the text after its codes. The match ignores case, so the codes are read
// from `text` itself.
fn parse_directive(text: &str) -> Option<(Directive, &str)> {
    let lower = text.to_ascii_lowercase();
    DIRECTIVES.into_iter().find_map(|(name, codes, scope)| {
        let (tool, action) = name.split_once(": ").map_or((None, name), |(tool, action)| (Some(tool), action));
        let after_tool = tool.map_or(Some(lower.as_str()), |tool| after_tool(&lower, tool));
        let rest = after_tool?.strip_prefix(action)?;
        if rest.starts_with(|char: char| char.is_alphanumeric() || matches!(char, '-' | '_')) {
            return None;
        }
        let (named, reason) = codes.split(&text[text.len() - rest.len()..]);
        let missing_codes = !named && !matches!(codes, Codes::None);
        Some((Directive { name, codes, scope, missing_codes }, reason))
    })
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
