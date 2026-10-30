use std::collections::HashMap;

use ruff_source_file::LineRanges;
use ruff_text_size::{TextRange, TextSize};

use crate::checker::Diagnostic;

pub fn filter(source: &str, comments: Vec<TextRange>, diagnostics: &mut Vec<Diagnostic>) {
    let directives: HashMap<TextSize, Directive> = comments
        .into_iter()
        .filter_map(|range| Some((source.line_start(range.start()), Directive::parse(&source[range])?)))
        .collect();
    diagnostics.retain(|diagnostic| {
        directives
            .get(&source.line_start(diagnostic.range.start()))
            .is_none_or(|directive| !directive.suppresses(diagnostic.rule.code()))
    });
}

enum Directive {
    All,
    Codes(Vec<String>),
}

impl Directive {
    fn parse(comment: &str) -> Option<Self> {
        let lower = comment.to_ascii_lowercase();
        let start = lower.match_indices('#').find_map(|(index, _)| {
            let rest = lower[index + 1..].trim_start();
            rest.starts_with("noqa").then(|| comment.len() - rest.len() + "noqa".len())
        })?;
        let Some(codes) = comment[start..].trim_start().strip_prefix(':') else {
            return Some(Self::All);
        };
        let codes: Vec<String> = codes
            .split(|char: char| char == ',' || char.is_whitespace())
            .filter(|part| !part.is_empty())
            .take_while(|part| is_code(part))
            .map(str::to_owned)
            .collect();
        Some(if codes.is_empty() { Self::All } else { Self::Codes(codes) })
    }

    fn suppresses(&self, code: &str) -> bool {
        match self {
            Self::All => true,
            Self::Codes(codes) => codes.iter().any(|candidate| candidate == code),
        }
    }
}

pub fn is_code(part: &str) -> bool {
    let letters = part.chars().take_while(char::is_ascii_uppercase).count();
    letters > 0 && part.len() > letters && part[letters..].chars().all(|char| char.is_ascii_digit())
}
