use std::collections::BTreeSet;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::noqa::is_noqa_for_only;
use crate::lint::registry::Rule;
use crate::rules::api::{exploded, is_type_checking};
use crate::rules::{export_key, indentation};
use crate::semantic::BindingKind;
use crate::syntax::is_main_guard;
use crate::syntax::{Exports, Visibility, alias_binding, all_binding, all_literal};

pub fn missing_from_all(checker: &Checker) {
    if checker.is_init_module()
        || checker.is_test_module()
        || checker.path.file_stem().is_some_and(|stem| stem == "__main__")
    {
        return;
    }
    let exported = match checker.dunder_all() {
        Exports::Absent => &[][..],
        Exports::Listed(names) => names.as_slice(),
        Exports::Dynamic => return,
    };
    // The package's `import *` of this module would bind its namesake over the attribute holding the module.
    let shadows_module = |name: &str| {
        checker.project().is_package_star_imported() && checker.path.file_stem().is_some_and(|stem| stem == name)
    };
    // A name bound only under `if TYPE_CHECKING:` does not exist at run time, so `import *` would fail on listing it.
    let type_checking: Vec<TextRange> = checker
        .body()
        .iter()
        .filter_map(|stmt| stmt.as_if_stmt().filter(|stmt| is_type_checking(&stmt.test, checker)))
        .filter_map(|stmt| Some(TextRange::new(stmt.body.first()?.start(), stmt.body.last()?.end())))
        .collect();
    let mut missing: Vec<(&str, TextRange, bool)> = checker
        .semantic()
        .all_bindings()
        .filter(|(name, bindings)| {
            Visibility::of(name) == Visibility::Public
                && !exported.contains(name)
                && !bindings.iter().all(|binding| type_checking.iter().any(|block| block.contains_range(binding.range)))
                // Another module reaching a name this one imports and uses, as `app.sys`, borrows it rather than
                // relying on an export; an import the module never uses is there for its importers, a re-export.
                && (!bindings.iter().any(|binding| binding.kind == BindingKind::Import)
                    || checker.semantic().references_to(name).is_empty())
                && ((checker.project().is_imported_by_others(name)
                    && !checker.project().is_used_only_by_tests(name))
                    || checker.project().is_api_reached(name))
                && !shadows_module(name)
        })
        .map(|(name, bindings)| (name, bindings[0].range, checker.project().is_imported_by_others(name)))
        .collect();
    if missing.is_empty() {
        return;
    }
    missing.sort_by_key(|(_, range, _)| range.start());
    // Listing a name in a public module of a library widens its API and what `import *` binds, so the report stays and
    // the author decides.
    let fix = if checker.project().role().public {
        None
    } else {
        all_insertion(checker, missing.iter().map(|(name, _, _)| *name))
            .and_then(|fix| with_unused_noqas_removed(checker, fix, &missing))
    };
    let names: Vec<String> = missing.iter().map(|(name, _, _)| format!("`{name}`")).collect();
    let title = format!("Add {} to `__all__`", names.join(", "));
    let offer_at = fix
        .as_ref()
        .and_then(|_| checker.first_unsuppressed(Rule::MissingFromAll, missing.iter().map(|(_, range, _)| *range)));
    // Repeating the whole listing in every report makes output slow; the first visible report carries it.
    let mut offer = fix.map(|fix| (fix, title));
    for (name, range, imported) in missing {
        let reason =
            if imported { "is imported by other modules" } else { "appears in the signature of a name in `__all__`" };
        let (fix, title) = if offer_at == Some(range) {
            offer.take().map_or((None, String::new()), |(fix, title)| (Some(fix), title))
        } else {
            (None, String::new())
        };
        checker.report_fix(Rule::MissingFromAll, range, format!("`{name}` {reason}; list it in `__all__`"), fix, title);
    }
}

// Listing a name silences Ruff's F401 for its import, so a `noqa` that named only F401 there is left unused, which Ruff
// reports as RUF100. The fix drops those comments with the listing.
fn with_unused_noqas_removed(checker: &Checker, fix: Fix, missing: &[(&str, TextRange, bool)]) -> Option<Fix> {
    let applicability = fix.applicability();
    let listed: FxHashSet<&str> = missing.iter().map(|(name, _, _)| *name).collect();
    // Names on one line share its comment, which is removed once: two deletions of it would overlap.
    let comments: BTreeSet<(TextSize, TextSize)> = missing
        .iter()
        .filter_map(|(_, range, _)| checker.trailing_comment(*range))
        .filter(|comment| is_noqa_for_only(checker.text(*comment), "F401"))
        .filter(|comment| binds_only_listed(checker, *comment, &listed))
        .map(|comment| (comment.start(), comment.end()))
        .collect();
    let unused = comments.into_iter().filter_map(|(start, end)| {
        let comment = TextRange::new(start, end);
        let before = checker.text(TextRange::new(checker.source.line_start(comment.start()), comment.start()));
        let code_end = comment.start() - TextSize::try_from(before.len() - before.trim_end().len()).ok()?;
        Some(Edit::range_deletion(TextRange::new(code_end, comment.end())))
    });
    let mut edits: Vec<Edit> = fix.into_edits().into_iter().chain(unused).collect();
    edits.sort_by_key(Ranged::start);
    let mut sorted = edits.into_iter();
    let first = sorted.next()?;
    Some(Fix::applicable_edits(first, sorted, applicability))
}

// Whether every name the imports on the comment's line bind is listed, so no import left unused needs the comment.
fn binds_only_listed(checker: &Checker, comment: TextRange, listed: &FxHashSet<&str>) -> bool {
    let line = checker.source.full_line_range(comment.start());
    checker
        .body()
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                Some(names)
            }
            _ => None,
        })
        .flatten()
        .filter(|alias| line.contains_range(alias.range()))
        .all(|alias| listed.contains(alias_binding(alias)))
}

fn all_insertion<'a>(checker: &Checker, names: impl Iterator<Item = &'a str>) -> Option<Fix> {
    let mut names: Vec<&str> = names.collect();
    names.sort_by_key(|name| export_key(name));
    let Some(stmt) = checker.body().iter().rev().find(|stmt| all_binding(stmt).is_some()) else {
        return new_all(checker, &names);
    };
    let (value, elts) = all_literal(stmt)?;
    let source = checker.source;
    let one_per_line =
        elts.iter().all(|elt| source[TextRange::new(source.line_start(elt.start()), elt.start())].trim().is_empty());
    let has_comment = checker.has_comment_in(value);
    if has_comment && elts.is_empty() {
        return None;
    }
    if source.contains_line_break(value) && !elts.is_empty() && (one_per_line || has_comment) {
        return lines_insertion(checker, stmt, (elts, one_per_line), &names);
    }
    let mut items: Vec<String> = elts.iter().map(|elt| checker.text(elt.range()).to_owned()).collect();
    for name in names.iter().rev() {
        items.insert(position(elts, name), quoted(checker, name));
    }
    let (open, close) = if source[value].starts_with('[') { ("[", "]") } else { ("(", ")") };
    let list = exploded(checker, indentation(source, stmt), &items);
    Some(Fix::safe_edit(Edit::range_replacement(format!("{open}{}{close}", &list[1..list.len() - 1]), value)))
}

// The `__all__` a module without one gets, last but above a closing `if __name__ == "__main__":` block, as SMP406 asks.
fn new_all(checker: &Checker, names: &[&str]) -> Option<Fix> {
    let source = checker.source;
    // An `import *` of this module takes every public name, which a new `__all__` would narrow to the listed ones.
    if checker.project().is_star_imported() {
        return None;
    }
    let items: Vec<String> = names.iter().map(|name| quoted(checker, name)).collect();
    let eol = checker.stylist.line_ending().as_str();
    let declaration = format!("__all__ = {}{eol}", exploded(checker, "", &items));
    // A module without `__all__` exports every public name to `from module import *`; adding one narrows that.
    let edit = if let Some(guard) = checker.body().last().filter(|stmt| is_main_guard(stmt)) {
        Edit::insertion(format!("{declaration}{eol}{eol}"), source.line_start(guard.start()))
    } else {
        let blank_lines = if source.ends_with('\n') { 2 } else { 3 };
        Edit::insertion(format!("{}{declaration}", eol.repeat(blank_lines)), TextSize::try_from(source.len()).ok()?)
    };
    Some(Fix::unsafe_edit(edit))
}

// Names one to a line, or after the last one when a comment pins the layout, as the `__all__` already lays them out.
fn lines_insertion(
    checker: &Checker,
    stmt: &Stmt,
    (elts, one_per_line): (&[Expr], bool),
    names: &[&str],
) -> Option<Fix> {
    let source = checker.source;
    let tail = &elts[elts.len() - 1];
    let indent = if one_per_line {
        indentation(source, &elts[0])
    } else {
        leading_indentation(source.full_line_str(tail.start()))
    };
    // A parenthesized element ends at its closing parenthesis, so the names go after that.
    let tail_range = checker.parenthesized_range(tail, stmt.into());
    // A comment ending the last element's line stays with that element only when the names go on the lines after it,
    // so nothing but a comma may come between them, and the closing bracket must follow the line.
    let line_comment = match checker
        .trailing_comment(tail_range)
        .map(|comment| source[TextRange::new(tail_range.end(), comment.start())].trim())
    {
        None => None,
        Some(",") => Some(false),
        Some("") if closes_after(checker, tail_range) => Some(true),
        Some(_) => return None,
    };
    // Names sort in the same order as their positions, so the ones sharing a position form one insertion.
    let mut groups: Vec<(usize, Vec<String>)> = Vec::new();
    for name in names {
        let at = if one_per_line { position(elts, name) } else { elts.len() };
        match groups.last_mut() {
            Some((last, group)) if *last == at => group.push(quoted(checker, name)),
            _ => groups.push((at, vec![quoted(checker, name)])),
        }
    }
    let eol = checker.stylist.line_ending().as_str();
    let separator = format!(",{eol}{indent}");
    let mut edits: Vec<Edit> = groups
        .into_iter()
        .map(|(at, group)| match elts.get(at) {
            Some(next) => Edit::insertion(
                group.iter().fold(String::new(), |text, name| text + name + &separator),
                checker.parenthesized_range(next, stmt.into()).start(),
            ),
            None if line_comment.is_some() => Edit::insertion(
                group.iter().fold(String::new(), |text, name| text + indent + name + "," + eol),
                source.full_line_end(tail_range.end()),
            ),
            None => Edit::insertion(
                group.iter().fold(String::new(), |text, name| text + &separator + name),
                tail_range.end(),
            ),
        })
        .collect();
    if line_comment == Some(true) {
        edits.push(Edit::insertion(",".to_owned(), tail_range.end()));
    }
    edits.sort_by_key(Ranged::start);
    let mut edits = edits.into_iter();
    let first = edits.next()?;
    Some(Fix::safe_edits(first, edits))
}

// Whether the closing bracket of the list is the next token after `elt`, past any comment or line break.
fn closes_after(checker: &Checker, elt: impl Ranged) -> bool {
    checker
        .parsed
        .tokens()
        .after(elt.end())
        .iter()
        .find(|token| !matches!(token.kind(), TokenKind::NonLogicalNewline | TokenKind::Comment))
        .is_some_and(|token| token.kind() == TokenKind::Rsqb)
}

fn quoted(checker: &Checker, name: &str) -> String {
    let quote = checker.stylist.quote().as_char();
    format!("{quote}{name}{quote}")
}

// Each name goes before the first listed name Ruff's RUF022 sorts after it, so a sorted `__all__` stays sorted.
fn position(elts: &[Expr], name: &str) -> usize {
    elts.iter()
        .position(|elt| {
            elt.as_string_literal_expr().is_some_and(|literal| export_key(literal.value.to_str()) > export_key(name))
        })
        .unwrap_or(elts.len())
}
