mod calls;
mod debug_text;
mod imports;
mod statements;

pub(super) use calls::argument_removal;
pub(super) use debug_text::{DebugFields, escape_braces, keep_debug_text};
pub(super) use imports::{Import, above_comments, ensure_import, preamble_len, with_import};
pub(super) use statements::{Blanks, statement_block, statement_deletion, statement_lines, statement_removal};

use ruff_diagnostics::{Applicability, Edit, Fix};

// The fix that applies the edits, in the order its constructor sorts them; none when there are no edits.
pub(super) fn fix_of(edits: impl IntoIterator<Item = Edit>, applicability: Applicability) -> Option<Fix> {
    let mut edits = edits.into_iter();
    let first = edits.next()?;
    Some(Fix::applicable_edits(first, edits, applicability))
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::{Applicability, Edit};
    use ruff_text_size::TextSize;

    use super::fix_of;

    #[test]
    fn fix_of_needs_an_edit() {
        let insertion = Edit::insertion("x".to_owned(), TextSize::from(0));
        assert!(fix_of(Vec::new(), Applicability::Safe).is_none());
        assert_eq!(
            fix_of([insertion.clone()], Applicability::Safe).map(|fix| fix.edits().to_vec()),
            Some(vec![insertion])
        );
    }
}
