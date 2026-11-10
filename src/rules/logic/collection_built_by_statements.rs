use ruff_diagnostics::{Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::comparable::HashableExpr;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::expr_mentions;
use crate::rules::logic::inline::{is_constant, is_hashable};
use crate::settings::FileSettings;
use crate::syntax::{for_each_body, is_name};

pub fn collection_built_by_statements(checker: &Checker) {
    for_each_body(checker.body(), &mut |body, _| {
        for (index, stmt) in body.iter().enumerate() {
            if let Some((name, kind)) = empty_collection(checker, stmt) {
                check_run(checker, stmt, name, kind, &body[index + 1..]);
            }
        }
    });
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Dict,
    List,
    Set,
}

// One statement that adds an entry: a key and value for a dict, a value for a list or a set.
struct Entry<'a> {
    stmt: &'a Stmt,
    key: Option<&'a Expr>,
    value: &'a Expr,
}

// The collection an empty literal or `set()` assigns to a name, and the kind of entries it takes.
fn empty_collection<'a>(checker: &Checker, stmt: &'a Stmt) -> Option<(&'a str, Kind)> {
    let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
        return None;
    };
    let [Expr::Name(target)] = targets.as_slice() else {
        return None;
    };
    let kind = match value.as_ref() {
        Expr::Dict(ast::ExprDict { items, .. }) if items.is_empty() => Kind::Dict,
        Expr::List(ast::ExprList { elts, .. }) if elts.is_empty() => Kind::List,
        Expr::Call(ast::ExprCall { func, arguments, .. })
            if arguments.args.is_empty()
                && arguments.keywords.is_empty()
                && checker.semantic().is_builtin(func, "set") =>
        {
            Kind::Set
        }
        _ => return None,
    };
    Some((target.id.as_str(), kind))
}

// The entry a statement adds to the collection, when it is one of the forms `d[key] = value`, `xs.append(value)` or
// `s.add(value)` for the collection's kind, and both key and value are constants or names that do not read the
// collection.
fn entry<'a>(stmt: &'a Stmt, name: &str, kind: Kind) -> Option<Entry<'a>> {
    let (key, value) = match (stmt, kind) {
        (Stmt::Assign(ast::StmtAssign { targets, value, .. }), Kind::Dict) => {
            let [Expr::Subscript(ast::ExprSubscript { value: target, slice, .. })] = targets.as_slice() else {
                return None;
            };
            if !is_name(target, name) {
                return None;
            }
            (Some(slice.as_ref()), value.as_ref())
        }
        (Stmt::Expr(ast::StmtExpr { value, .. }), Kind::List | Kind::Set) => {
            let Expr::Call(ast::ExprCall { func, arguments, .. }) = value.as_ref() else {
                return None;
            };
            let Expr::Attribute(ast::ExprAttribute { value: receiver, attr, .. }) = func.as_ref() else {
                return None;
            };
            let method = if kind == Kind::List { "append" } else { "add" };
            let [argument] = arguments.args.as_ref() else {
                return None;
            };
            if !is_name(receiver, name) || attr.as_str() != method || !arguments.keywords.is_empty() {
                return None;
            }
            (None, argument)
        }
        _ => return None,
    };
    let key_ok = key.is_none_or(|key| is_hashable(key) || is_name_expr(key));
    let value_ok = if kind == Kind::Set { is_hashable(value) } else { is_constant(value) } || is_name_expr(value);
    let names_ok = key.is_none_or(|key| !expr_mentions(key, name)) && !expr_mentions(value, name);
    (key_ok && value_ok && names_ok).then_some(Entry { stmt, key, value })
}

fn is_name_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Name(_))
}

// The run of statements right after the literal that each add an entry, which the literal can hold instead.
fn check_run(checker: &Checker, assign: &Stmt, name: &str, kind: Kind, rest: &[Stmt]) {
    if kind == Kind::List && ruff_repeats_append(checker, name, rest) {
        return;
    }
    let entries: Vec<Entry> = rest.iter().map_while(|stmt| entry(stmt, name, kind)).collect();
    let Some(last) = entries.last() else {
        return;
    };
    let range = TextRange::new(assign.start(), last.stmt.end());
    // A comment in the run would go with the statements it stands between.
    if checker.has_comment_in(range) || has_repeated_entry(&entries, kind) {
        return;
    }
    // A name may be unbound, and a key or element may be unhashable, so only a run of constants can raise nothing,
    // and only then do the statements and the literal raise at the same point.
    let constant = entries.iter().all(|entry| entry.key.is_none_or(is_hashable) && is_constant(entry.value));
    let text = format!("{name} = {}", literal(checker, kind, &entries));
    let fix = if constant {
        Fix::safe_edit(Edit::range_replacement(text, range))
    } else {
        Fix::unsafe_edit(Edit::range_replacement(text, range))
    };
    let count = entries.len();
    let noun = if count == 1 { "statement" } else { "statements" };
    checker.report_fix(
        Rule::CollectionBuiltByStatements,
        assign.range(),
        format!("`{name}` is filled by {count} {noun} right after its literal; write its entries in the literal"),
        Some(fix),
        "Write the entries in the literal",
    );
}

// Whether the rule runs on a file where Ruff's FURB113 counts list appends, which the rule then reads from the semantic
// model.
pub(crate) fn counts_ruff_appends(settings: &FileSettings) -> bool {
    settings.rules.contains(Rule::CollectionBuiltByStatements)
        && settings.from_ruff.preview
        && settings.from_ruff.ruff_runs(RuffRule::RepeatedAppend)
}

// Ruff's FURB113 reports two or more appends to a list in a run of consecutive appends, when preview mode is on and the
// project selects it. The run counts every append to the list in the appends that follow the literal, not only the
// ones that enter it here.
fn ruff_repeats_append(checker: &Checker, name: &str, rest: &[Stmt]) -> bool {
    counts_ruff_appends(checker.settings)
        && rest
            .iter()
            .take_while(|stmt| checker.semantic().is_list_append(stmt.range()))
            .filter(|stmt| appends_to(stmt, name))
            .count()
            >= 2
}

fn appends_to(stmt: &Stmt, name: &str) -> bool {
    stmt.as_expr_stmt()
        .and_then(|expr| expr.value.as_call_expr())
        .and_then(|call| call.func.as_attribute_expr())
        .is_some_and(|attribute| is_name(&attribute.value, name))
}

// Two keys of a dict, or two elements of a set, that hash alike as Python hashes them, such as `1` and `True`, are one
// entry. The literal would repeat it, which Ruff reports (F601, F602, B033), so the run is left as written.
fn has_repeated_entry(entries: &[Entry], kind: Kind) -> bool {
    let mut seen = FxHashSet::default();
    entries
        .iter()
        .filter_map(|entry| match kind {
            Kind::Dict => entry.key,
            Kind::Set => Some(entry.value),
            Kind::List => None,
        })
        .any(|key| !seen.insert(HashableExpr::from(key)))
}

// The collection's literal with the entries in the order the statements added them. A tuple that is not in parentheses
// in the source is parenthesized, since a comma would end the entry.
fn literal(checker: &Checker, kind: Kind, entries: &[Entry]) -> String {
    let text = |expr: &Expr| {
        let source = checker.text(expr);
        match expr {
            Expr::Tuple(ast::ExprTuple { parenthesized: false, .. }) => format!("({source})"),
            _ => source.to_owned(),
        }
    };
    let items: Vec<String> = entries
        .iter()
        .map(|entry| match entry.key {
            Some(key) => format!("{}: {}", text(key), text(entry.value)),
            None => text(entry.value),
        })
        .collect();
    let items = items.join(", ");
    match kind {
        Kind::Dict | Kind::Set => format!("{{{items}}}"),
        Kind::List => format!("[{items}]"),
    }
}
