use ruff_diagnostics::{Edit, Fix};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::exports::exploded;
use crate::rules::exports::{Exports, all_binding, all_literal};
use crate::rules::{indentation, is_main_guard};

pub fn missing_from_all(checker: &Checker) {
    if checker.is_init_module() || checker.is_test_module() {
        return;
    }
    let exported = match checker.dunder_all() {
        Exports::Absent => &[][..],
        Exports::Listed(names) => names.as_slice(),
        Exports::Dynamic => return,
    };
    let mut missing: Vec<(&str, TextRange)> = checker
        .semantic()
        .all_bindings()
        .filter(|(name, _)| !name.starts_with('_') && !exported.contains(name) && checker.project().is_imported(name))
        .map(|(name, bindings)| (name, bindings[0].range))
        .collect();
    if missing.is_empty() {
        return;
    }
    missing.sort_by_key(|(_, range)| range.start());
    let fix = all_insertion(checker, missing.iter().map(|(name, _)| *name));
    let names: Vec<String> = missing.iter().map(|(name, _)| format!("`{name}`")).collect();
    let title = format!("Add {} to `__all__`", names.join(", "));
    for (name, range) in missing {
        checker.report_fix(
            Rule::MissingFromAll,
            range,
            format!("`{name}` is imported by other modules; list it in `__all__`"),
            fix.clone(),
            title.clone(),
        );
    }
}

fn all_insertion<'a>(checker: &Checker, names: impl Iterator<Item = &'a str>) -> Option<Fix> {
    let quote = checker.stylist.quote().as_char();
    let mut names: Vec<&str> = names.collect();
    names.sort_by_key(|name| export_key(name));
    let quoted = |name: &str| format!("{quote}{name}{quote}");
    let body = checker.body();
    let source = checker.source;
    let Some(stmt) = body.iter().rev().find(|stmt| all_binding(stmt).is_some()) else {
        let items: Vec<String> = names.iter().map(|name| quoted(name)).collect();
        let declaration = format!("__all__ = {}\n", exploded(checker, "", &items));
        // A module without `__all__` exports every public name to `from module import *`; adding one narrows that.
        // It goes last, but above a closing `if __name__ == "__main__":` block, as SMP104 asks.
        let edit = if let Some(guard) = body.last().filter(|stmt| is_main_guard(stmt)) {
            Edit::insertion(format!("{declaration}\n\n"), source.line_start(guard.start()))
        } else {
            let separator = if source.ends_with('\n') { "\n\n" } else { "\n\n\n" };
            Edit::insertion(format!("{separator}{declaration}"), TextSize::try_from(source.len()).ok()?)
        };
        return Some(Fix::unsafe_edit(edit));
    };
    let (value, elts) = all_literal(stmt)?;
    // Each name goes before the first listed name Ruff's RUF022 sorts after it, so a sorted `__all__` stays sorted.
    let position = |name: &str| {
        elts.iter()
            .position(|elt| {
                elt.as_string_literal_expr()
                    .is_some_and(|literal| export_key(literal.value.to_str()) > export_key(name))
            })
            .unwrap_or(elts.len())
    };
    let one_per_line =
        elts.iter().all(|elt| source[TextRange::new(source.line_start(elt.start()), elt.start())].trim().is_empty());
    let has_comment = checker.has_comment_in(value);
    if source.contains_line_break(value) && !elts.is_empty() && (one_per_line || has_comment) {
        let indent = if one_per_line {
            indentation(source, &elts[0])
        } else {
            leading_indentation(source.full_line_str(elts[elts.len() - 1].start()))
        };
        // Without one name per line, a comment pins the layout, so the names go after the last one.
        let position = |name: &str| if one_per_line { position(name) } else { elts.len() };
        // Names sort in the same order as their positions, so the ones sharing a position form one insertion.
        let mut groups: Vec<(usize, Vec<String>)> = Vec::new();
        for name in &names {
            let at = position(name);
            match groups.last_mut() {
                Some((last, group)) if *last == at => group.push(quoted(name)),
                _ => groups.push((at, vec![quoted(name)])),
            }
        }
        let mut edits = groups.into_iter().map(|(at, group)| match elts.get(at) {
            Some(next) => Edit::insertion(
                group.iter().fold(String::new(), |text, name| text + name + ",\n" + indent),
                next.start(),
            ),
            None => Edit::insertion(
                group.iter().fold(String::new(), |text, name| text + ",\n" + indent + name),
                elts[elts.len() - 1].end(),
            ),
        });
        let first = edits.next()?;
        return Some(Fix::safe_edits(first, edits));
    }
    let mut items: Vec<String> = elts.iter().map(|elt| checker.text(elt.range()).to_owned()).collect();
    for name in names.iter().rev() {
        items.insert(position(name), quoted(name));
    }
    let (open, close) = if source[value].starts_with('[') { ("[", "]") } else { ("(", ")") };
    let list = exploded(checker, indentation(source, stmt), &items);
    Some(Fix::safe_edit(Edit::range_replacement(format!("{open}{}{close}", &list[1..list.len() - 1]), value)))
}

// Ruff's RUF022 sorts `__all__` isort-style: SCREAMING_SNAKE_CASE names first, then CamelCase, then the rest.
fn export_key(name: &str) -> (u8, String) {
    let rank = if name.chars().any(char::is_alphabetic) && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_owned())
}
