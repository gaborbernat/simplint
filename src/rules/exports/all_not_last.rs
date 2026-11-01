use ruff_diagnostics::{Edit, Fix};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::all_binding;
use crate::rules::{is_alone_on_lines, is_main_guard, lines_with_trailing_blanks};

pub fn all_not_last(checker: &Checker) {
    let body = checker.body();
    let positions: Vec<usize> =
        body.iter().enumerate().filter(|(_, stmt)| all_binding(stmt).is_some()).map(|(index, _)| index).collect();
    let Some(&last) = positions.last() else {
        return;
    };
    let end = body.len() - usize::from(body.last().is_some_and(is_main_guard));
    if last + 1 >= end {
        return;
    }
    let stmt = &body[last];
    let source = checker.source;
    // A later `__all__ += [...]` inside a block extends the list, so it must still find it bound above.
    let semantic = checker.semantic();
    let referenced_later = semantic.references_to("__all__").iter().any(|reference| reference.statement > last)
        || semantic.bindings("__all__").iter().any(|binding| binding.range.start() > stmt.end());
    let fix = (positions.len() == 1 && !referenced_later && is_alone_on_lines(source, stmt.range())).then(|| {
        // A `# fmt: off` / `# fmt: on` pair around `__all__` keeps `ruff format` off its layout, which can hold a
        // suppression comment for several names on one line, so the pair moves with it.
        let after = source.full_line_end(stmt.end());
        let fence = checker.comment_above(stmt).filter(|comment| {
            source[*comment].trim() == "# fmt: off" && source.full_line_str(after).trim() == "# fmt: on"
        });
        let lines =
            lines_with_trailing_blanks(source, fence.map_or(stmt.range(), |off| TextRange::new(off.start(), after)));
        let insertion = source.full_line_end(body[end - 1].end());
        let text = source[lines].trim_end();
        let content = if insertion.to_usize() == source.len() && !source.ends_with('\n') {
            format!("\n\n\n{text}")
        } else {
            format!("\n\n{text}\n")
        };
        Fix::safe_edits(Edit::range_deletion(lines), [Edit::insertion(content, insertion)])
    });
    checker.report_fix(
        Rule::AllNotLast,
        stmt.range(),
        "`__all__` is not the last statement; move it to the end of the module",
        fix,
        "Move `__all__` to the end of the module",
    );
}
