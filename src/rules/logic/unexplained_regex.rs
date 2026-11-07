use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::regex_pattern::{Facts, scan};

// Shorter patterns read at a glance; a reader needs a note from four constructs on.
const MIN_CONSTRUCTS: usize = 4;

pub(crate) fn unexplained_regex(checker: &Checker) {
    let facts = Facts::collect(checker);
    for (literal, call) in facts.sites() {
        let Some(scan) = scan(&literal.decoded(), call.verbose) else {
            continue;
        };
        // A verbose flag alone explains no part, so only a comment satisfies the rule.
        if scan.constructs < MIN_CONSTRUCTS || scan.comments > 0 {
            continue;
        }
        let parts = scan.constructs;
        let message = if scan.verbose {
            format!("regex has {parts} parts and no comments; put a `#` comment on each part a reader needs explained")
        } else {
            format!(
                "regex has {parts} parts and no comments; write it with `re.VERBOSE`, one part per line with a `#` comment"
            )
        };
        checker.report(Rule::UnexplainedRegex, literal.range, message);
    }
}
