use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{
    Exports, all_literal, dunder_all, indentation, is_all_target, is_alone_on_lines, is_main_guard,
    lines_with_trailing_blanks,
};

pub fn all_not_last(checker: &mut Checker) {
    let body = checker.body();
    let positions: Vec<usize> =
        body.iter().enumerate().filter(|(_, stmt)| binds_all(stmt)).map(|(index, _)| index).collect();
    let Some(&last) = positions.last() else {
        return;
    };
    let end = body.len() - usize::from(body.last().is_some_and(is_main_guard));
    if last + 1 >= end {
        return;
    }
    let stmt = &body[last];
    let source = checker.source;
    let referenced_later = checker.semantic().references_to("__all__").any(|reference| reference.statement > last);
    let fix = (positions.len() == 1 && !referenced_later && is_alone_on_lines(source, stmt.range())).then(|| {
        let lines = lines_with_trailing_blanks(source, stmt.range());
        let insertion = source.full_line_end(body[end - 1].end());
        let text = source[lines].trim_end();
        let content = if insertion.to_usize() == source.len() && !source.ends_with('\n') {
            format!("\n\n{text}")
        } else {
            format!("\n\n{text}\n")
        };
        Fix::safe_edits(Edit::range_deletion(lines), [Edit::insertion(content, insertion)])
    });
    checker.report(
        Rule::AllNotLast,
        stmt.range(),
        "`__all__` is not the last statement; move it to the end of the module",
        fix,
    );
}

fn binds_all(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().any(is_all_target),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            is_all_target(target)
        }
        _ => false,
    }
}

pub fn missing_from_all(checker: &mut Checker) {
    if checker.is_init_module() || checker.is_test_module() {
        return;
    }
    let body = checker.body();
    let exported = match dunder_all(body) {
        Exports::Absent => Vec::new(),
        Exports::Listed(names) => names,
        Exports::Dynamic => return,
    };
    let mut missing: Vec<(&str, TextRange)> = checker
        .semantic()
        .bindings
        .iter()
        .filter(|(name, _)| !name.starts_with('_') && !exported.contains(name) && checker.is_imported_elsewhere(name))
        .map(|(name, bindings)| (*name, bindings[0].range))
        .collect();
    if missing.is_empty() {
        return;
    }
    missing.sort_by_key(|(_, range)| range.start());
    let fix = all_insertion(checker, missing.iter().map(|(name, _)| *name));
    for (name, range) in missing {
        checker.report(
            Rule::MissingFromAll,
            range,
            format!("`{name}` is imported by other modules; list it in `__all__`"),
            fix.clone(),
        );
    }
}

fn all_insertion<'a>(checker: &Checker, names: impl Iterator<Item = &'a str>) -> Option<Fix> {
    let quote = checker.stylist.quote().as_char();
    let quoted: Vec<String> = names.map(|name| format!("{quote}{name}{quote}")).collect();
    let body = checker.body();
    let Some(stmt) = body.iter().rev().find(|stmt| binds_all(stmt)) else {
        let source = checker.source;
        let separator = if source.ends_with('\n') { "\n\n" } else { "\n\n\n" };
        let insertion = TextSize::try_from(source.len()).ok()?;
        return Some(Fix::safe_edit(Edit::insertion(
            format!("{separator}__all__ = [{}]\n", quoted.join(", ")),
            insertion,
        )));
    };
    let (value, elts) = all_literal(stmt)?;
    let edit = match elts.last() {
        Some(last) if checker.source.contains_line_break(value) => {
            let indent = indentation(checker.source, last);
            let lines: Vec<String> = quoted.iter().map(|name| format!(",\n{indent}{name}")).collect();
            Edit::insertion(lines.concat(), last.end())
        }
        Some(last) => Edit::insertion(format!(", {}", quoted.join(", ")), last.end()),
        None => Edit::insertion(quoted.join(", "), value.end() - TextSize::from(1)),
    };
    Some(Fix::safe_edit(edit))
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::AllNotLast, "SMP104.py" ; "all_not_last")]
    #[test_case(Rule::AllNotLast, "SMP104_eof.py" ; "all_not_last_at_eof")]
    #[test_case(Rule::AllNotLast, "SMP104_reversed.py" ; "all_not_last_reversed_main_guard")]
    #[test_case(Rule::AllNotLast, "SMP104_negated.py" ; "all_not_last_negated_guard")]
    #[test_case(Rule::AllNotLast, "SMP104_nonl.py" ; "all_not_last_without_trailing_newline")]
    #[test_case(Rule::AllNotLast, "SMP104_ann.py" ; "all_not_last_annotated_and_augmented")]
    #[test_case(Rule::AllNotLast, "SMP003_ok.py" ; "all_not_last_without_all")]
    #[test_case(Rule::MissingFromAll, "SMP113.py" ; "missing_from_all")]
    #[test_case(Rule::MissingFromAll, "SMP113_multiline.py" ; "missing_from_all_multiline")]
    #[test_case(Rule::MissingFromAll, "SMP113_noall.py" ; "missing_from_all_creates_all")]
    #[test_case(Rule::MissingFromAll, "SMP113_aug.py" ; "missing_from_all_augmented")]
    #[test_case(Rule::MissingFromAll, "SMP113_call.py" ; "missing_from_all_dynamic")]
    #[test_case(Rule::MissingFromAll, "SMP113_empty.py" ; "missing_from_all_empty")]
    #[test_case(Rule::MissingFromAll, "SMP113_scripts.py" ; "missing_from_all_entry_point")]
    #[test_case(Rule::MissingFromAll, "tests/test_SMP004.py" ; "missing_from_all_test_module")]
    #[test_case(Rule::MissingFromAll, "package/__init__.py" ; "missing_from_all_init_module")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
