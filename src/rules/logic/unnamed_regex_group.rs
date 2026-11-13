use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::fix_of;
use crate::rules::logic::regex_pattern::{Context, scan};

const MATCH_CALLS: [&str; 6] = ["compile", "match", "prefixmatch", "search", "fullmatch", "finditer"];

pub fn unnamed_regex_group(checker: &Checker) {
    let facts = checker.state().regex_facts(checker);
    for (literal, call) in facts.sites() {
        let Some(scan) = scan(&literal.decoded(), call.verbose) else {
            continue;
        };
        let count = scan.groups.len();
        // A named group takes a number too, so with one present a lone unnamed group's number may not be the one the
        // code reads.
        let reported = count >= 2
            || (count == 1
                && !scan.named
                && MATCH_CALLS.contains(&call.attr)
                && matches!(call.context, Context::Bind { name, scope } if facts.reads_index(scope, name)));
        if !reported || facts.positional_only(call) {
            continue;
        }
        let fix = (!scan.refers && facts.call_flows(checker, call))
            .then(|| insertions(checker, &literal.parts, &scan.groups))
            .flatten();
        let message = if count == 1 {
            "regex has 1 unnamed capture group that the code reads by index; name it with `(?P<name>...)` so readers \
             do not count parentheses"
                .to_owned()
        } else {
            format!(
                "regex has {count} unnamed capture groups; name them with `(?P<name>...)` so readers \
                 do not count parentheses"
            )
        };
        checker.report_fix(
            Rule::UnnamedRegexGroup,
            literal.range,
            message,
            fix,
            "Make the unnamed groups non-capturing",
        );
    }
}

// Each insertion lands in the source only when every part's source text is its decoded value, so a part with an escape
// leaves the fix out rather than misplacing it.
fn insertions(checker: &Checker, parts: &[(&[u8], TextRange)], groups: &[usize]) -> Option<Fix> {
    let mut starts = Vec::with_capacity(parts.len());
    let mut decoded = 0;
    for (value, content) in parts {
        if checker.text(*content).as_bytes() != *value {
            return None;
        }
        starts.push((decoded, content.start()));
        decoded += value.len();
    }
    let edits = groups.iter().map(|&group| {
        let (part_start, source_start) = starts[starts.partition_point(|(start, _)| *start <= group) - 1];
        let offset = TextSize::try_from(group - part_start + 1).expect("a group sits in the text of its part");
        Edit::insertion("?:".to_owned(), source_start + offset)
    });
    fix_of(edits, Applicability::Unsafe)
}
