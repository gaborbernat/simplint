use ruff_python_trivia::{indentation_at_offset, is_pragma_comment};
use ruff_text_size::TextRange;

use crate::lint::noqa::{after_tool, noqa_codes};

// The suppression directives SMP003 and SMP004 read, as the table spells them, how each names its codes and what it
// covers.
const DIRECTIVES: [(&str, Codes, Scope); 12] = [
    ("noqa", Codes::Colon, Scope::Line),
    ("ruff: noqa", Codes::Colon, Scope::File),
    ("flake8: noqa", Codes::Colon, Scope::File),
    ("ruff: ignore", Codes::Bracket, Scope::Line),
    ("ruff: file-ignore", Codes::Bracket, Scope::Module),
    ("simplint: ignore", Codes::Bracket, Scope::Line),
    ("type: ignore", Codes::Bracket, Scope::Line),
    ("ty: ignore", Codes::Bracket, Scope::Line),
    ("pyright: ignore", Codes::Bracket, Scope::Line),
    ("pyrefly: ignore", Codes::Bracket, Scope::Line),
    ("pragma: no cover", Codes::None, Scope::Line),
    ("pragma: no branch", Codes::None, Scope::Line),
];

// Tools read these comment prefixes as directives, such as `# fmt: off`, beyond the pragmas Ruff knows.
const TOOL_PREFIXES: [&str; 3] = ["fmt:", "mypy:", "simplint:"];

// Whether a comment is a directive for a suppression or a tool, which a rule must not write or reflow, so its words are
// not prose.
pub(crate) fn is_tool_directive(comment: &str) -> bool {
    let text = comment.trim_start_matches('#').trim_start();
    is_pragma_comment(comment)
        || !Comment::parse(comment).directives.is_empty()
        || TOOL_PREFIXES.iter().any(|prefix| text.starts_with(prefix))
}

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
    // A file-level `noqa`, which Ruff applies only on a line of its own.
    File,
    // A `file-ignore`, which Ruff applies only at module scope.
    Module,
}

impl Comment {
    // The comment at `range`, without the file-level directives its place keeps Ruff from applying.
    pub(crate) fn at(source: &str, range: TextRange) -> Self {
        let mut comment = Self::parse(&source[range]);
        let indentation = indentation_at_offset(range.start(), source);
        comment.directives.retain(|directive| match directive.scope {
            Scope::Line => true,
            Scope::File => indentation.is_some(),
            Scope::Module => indentation == Some(""),
        });
        comment
    }

    fn parse(comment: &str) -> Self {
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
        // Ruff reads a tool name in lowercase only, so `# RUFF: noqa` and `# TY : IGNORE` are no directives, while
        // coverage.py's default exclusion also takes `PRAGMA` in capitals.
        if let Some(tool) = tool
            && !text.starts_with(tool)
            && !(tool == "pragma" && text.starts_with("PRAGMA"))
        {
            return None;
        }
        let after_tool = tool.map_or(Some(lower.as_str()), |tool| after_tool(&lower, tool));
        let rest = after_tool?.strip_prefix(action)?;
        if rest.starts_with(|char: char| char.is_alphanumeric() || matches!(char, '-' | '_')) {
            return None;
        }
        let (named, reason) = codes.split(&text[text.len() - rest.len()..])?;
        let missing_codes = !named && !matches!(codes, Codes::None);
        Some((Directive { name, codes, scope, missing_codes }, reason))
    })
}

impl Codes {
    // Whether the directive names codes, and the text after them. `None` for a `noqa` Ruff rejects, which is no
    // directive.
    fn split(self, rest: &str) -> Option<(bool, &str)> {
        match self {
            Self::Colon => noqa_codes(rest).map(|(codes, reason)| (!codes.is_empty(), reason)),
            Self::Bracket => Some(match rest.strip_prefix('[').and_then(|inner| inner.split_once(']')) {
                Some((codes, reason)) => (!codes.trim().is_empty(), reason),
                None => (false, rest),
            }),
            Self::None => Some((true, rest)),
        }
    }
}
