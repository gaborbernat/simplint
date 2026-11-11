use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};
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
        let mut seen: FxHashSet<(u8, Option<&str>)> = FxHashSet::default();
        let mut reported_placement = false;
        for field in &docstring.fields {
            let range = field.range_in(docstring);
            let text = field.text();
            let word = &field.words[0];
            if field.joins_text {
                // A blank line before the field separates the field list from the paragraph above it.
                let fix = field.lines.map(|lines| {
                    Fix::unsafe_edit(Edit::insertion(checker.stylist.line_ending().as_str().to_owned(), lines.start()))
                });
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
            if let Some(primary) = primary_name(word) {
                let fix = field.marker.map(|marker| {
                    let name = TextRange::at(marker.start(), TextSize::of(word.as_str()));
                    Fix::unsafe_edit(Edit::range_replacement(primary.to_owned(), name))
                });
                checker.report_fix(
                    Rule::RstFieldSyntax,
                    range,
                    format!("Field `{text}` uses the alias `{word}`; write `{primary}`"),
                    fix,
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
            if !repeatable && !seen.insert((field.kind as u8, field.subject())) {
                checker.report(Rule::RstFieldSyntax, range, format!("Field `{text}` repeats an earlier field"));
            }
        }
    }
}

// The name an alias of a field stands for, as Sphinx reads it. `var`, `ivar` and `cvar` stay apart: Sphinx renders them
// alike, yet `rst-attribute-kind` tells instance attributes from class variables by them.
fn primary_name(name: &str) -> Option<&'static str> {
    match name {
        "parameter" | "arg" | "argument" | "key" | "keyword" | "kwarg" | "kwparam" => Some("param"),
        "paramtype" => Some("type"),
        "raise" | "except" | "exception" => Some("raises"),
        "return" => Some("returns"),
        "yield" => Some("yields"),
        _ => None,
    }
}
