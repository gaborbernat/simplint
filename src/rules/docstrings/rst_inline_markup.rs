use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rst::{Cause, bare_roles, classify_docstring, literal_findings};

pub fn rst_inline_markup(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        // Positions in the source hold only for a docstring whose text is its source.
        let Some(content) = target.docstring.content else {
            continue;
        };
        let text = &checker.source[content];
        let at = |offset: usize| content.start() + TextSize::try_from(offset).unwrap_or_default();
        let lines = classify_docstring(text);
        for role in bare_roles(&lines) {
            checker.report_fix(
                Rule::RstInlineMarkup,
                TextRange::new(at(role.start), at(role.end)),
                format!(
                    "Role `{}` has no backticks around its target, so it renders as plain text",
                    &text[role.start..role.end]
                ),
                role.fixable.then(|| {
                    Fix::unsafe_edits(
                        Edit::insertion("`".to_owned(), at(role.target)),
                        [Edit::insertion("`".to_owned(), at(role.end))],
                    )
                }),
                "Put the target in backticks",
            );
        }
        for finding in literal_findings(&lines) {
            let message = match &finding.cause {
                Cause::Unclosed => concat!(
                    "Inline literal opened with ``` `` ``` is not closed in its paragraph, ",
                    "so the backticks and the rest show as plain text, with a warning"
                )
                .to_owned(),
                Cause::Suffix { literal, suffix } => {
                    format!("Text right after ``{literal}`` keeps the literal open; write ``{literal}``\\ {suffix}")
                }
                Cause::Fence => "Markdown code fence in a reST docstring; use `::` or `.. code-block::`".to_owned(),
                Cause::Mismatch { opened, closed } => format!(
                    "Mismatched backticks: opened with {} and closed with {}",
                    "`".repeat(*opened),
                    "`".repeat(*closed)
                ),
            };
            checker.report(
                Rule::RstInlineMarkup,
                TextRange::new(at(finding.at), at(finding.at + finding.len)),
                message,
            );
        }
    }
}
