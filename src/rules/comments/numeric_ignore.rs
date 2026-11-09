use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::noqa::ignore_contents;
use crate::lint::registry::Rule;

pub fn numeric_ignore(checker: &Checker) {
    for (comment, _) in checker.state().comments(checker) {
        let Some((offset, codes)) = ignore_contents(checker.text(*comment)) else {
            continue;
        };
        let mut replacement = String::with_capacity(codes.len());
        let mut changed = false;
        for part in codes.split_inclusive(|char: char| char == ',' || char.is_whitespace()) {
            let code = part.trim_end_matches(|char: char| char == ',' || char.is_whitespace());
            if let Some(rule) = Rule::from_code(code) {
                replacement.push_str(rule.name());
                replacement.push_str(&part[code.len()..]);
                changed = true;
            } else {
                replacement.push_str(part);
            }
        }
        if !changed {
            continue;
        }
        let start = comment.start() + offset;
        let range = TextRange::new(start, start + TextSize::of(codes));
        checker.report_fix(
            Rule::NumericIgnore,
            range,
            "Numeric simplint ignore; use rule names",
            Some(Fix::safe_edit(Edit::range_replacement(replacement, range))),
            "Replace numeric codes with rule names",
        );
    }
}
