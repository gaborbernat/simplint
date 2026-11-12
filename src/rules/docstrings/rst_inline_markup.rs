use ruff_diagnostics::{Edit, Fix};
use ruff_text_size::{TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rst::{bare_roles, classify_docstring, unclosed_literals};

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
        for opening in unclosed_literals(&lines) {
            checker.report(
                Rule::RstInlineMarkup,
                TextRange::new(at(opening), at(opening + 2)),
                "Inline literal opened with ``` `` ``` is not closed in its paragraph, so the rest renders as code",
            );
        }
    }
}
