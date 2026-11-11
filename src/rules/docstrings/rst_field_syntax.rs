use ruff_diagnostics::{Edit, Fix};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::FieldKind;

pub fn rst_field_syntax(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        let docstring = &target.docstring;
        for section in &docstring.sections {
            let message = if section.numpy {
                format!(
                    "numpydoc section `{}` in a reStructuredText docstring; write fields such as `:param name:`",
                    section.heading
                )
            } else {
                format!(
                    "Google-style section `{}:` in a reStructuredText docstring; write fields such as `:param name:`",
                    section.heading
                )
            };
            checker.report(Rule::RstFieldSyntax, docstring.range(), message);
        }
        let mut seen: FxHashSet<(FieldKind, Option<&str>)> = FxHashSet::default();
        let mut reported_placement = false;
        for field in &docstring.fields {
            let range = field.range_in(docstring);
            let text = field.text();
            let word = &field.words[0];
            if field.joins_text {
                // A blank line before the field separates the field list from the paragraph above it.
                let fix = field
                    .lines
                    .map(|lines| Fix::unsafe_edit(Edit::insertion(checker.line_ending().to_owned(), lines.start())));
                checker.report_fix(
                    Rule::RstFieldSyntax,
                    range,
                    format!(
                        "Field `{text}` follows text without a blank line, so it renders as part of that paragraph"
                    ),
                    fix,
                    "Add a blank line before the field list",
                );
            }
            if let Some(primary) = field.primary {
                checker.report_fix(
                    Rule::RstFieldSyntax,
                    range,
                    format!("Field `{text}` uses the alias `{word}`; write `{primary}`"),
                    field.rename(primary),
                    format!("Rename the field to `:{primary}:`"),
                );
            }
            if field.after_text && !reported_placement {
                reported_placement = true;
                checker.report(
                    Rule::RstFieldSyntax,
                    range,
                    format!("Field `{text}` follows text after the field list; keep the fields together at the end"),
                );
            }
            if let Some(problem) = field.problem() {
                checker.report(Rule::RstFieldSyntax, range, problem);
                continue;
            }
            // `:raises:` may name one exception twice, for each of the conditions that raise it.
            let repeatable = matches!(field.kind, FieldKind::Raises | FieldKind::Meta | FieldKind::Unknown);
            if !repeatable && !seen.insert((field.kind, field.subject())) {
                checker.report(Rule::RstFieldSyntax, range, format!("Field `{text}` repeats an earlier field"));
            }
        }
    }
}
