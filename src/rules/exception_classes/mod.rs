mod hierarchy;

use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::syntax::{last_dotted_segment, last_segment};

use self::hierarchy::{ALIASES, BASES};

// An exception class by identity: a class of the standard hierarchy table, or a class this file defines.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ClassKey {
    Known(&'static str),
    Local(TextRange),
}

// Every class `expr` derives from, itself first; `None` when `expr` names no class the table or this file knows. A
// local class with a base that cannot be resolved keeps the bases that can, so a class found in it is a real ancestor.
pub(crate) fn lineage(checker: &Checker, expr: &Expr) -> Option<Vec<ClassKey>> {
    let mut found = Vec::new();
    collect(checker, expr, &mut found).then_some(found)
}

fn collect(checker: &Checker, expr: &Expr, found: &mut Vec<ClassKey>) -> bool {
    if let Some(class) = local_class(checker, expr) {
        let key = ClassKey::Local(class.range());
        if !found.contains(&key) {
            found.push(key);
            for base in class.bases() {
                collect(checker, base, found);
            }
        }
        return true;
    }
    let Some(name) = known_name(checker, expr) else {
        return false;
    };
    let mut pending = vec![name];
    while let Some(name) = pending.pop() {
        let key = ClassKey::Known(name);
        if found.contains(&key) {
            continue;
        }
        found.push(key);
        pending.extend(lookup(BASES, name).into_iter().flat_map(|(_, bases)| bases.iter().rev()).copied());
    }
    true
}

// The class a name binds in this file, when that is its only binding there.
fn local_class<'a>(checker: &Checker<'a>, expr: &Expr) -> Option<&'a ast::StmtClassDef> {
    let name = expr.as_name_expr()?;
    let bindings = checker.semantic().resolved_bindings(name);
    let [binding] = bindings.as_slice() else {
        return None;
    };
    checker.semantic().class_of_binding(binding)
}

fn known_name(checker: &Checker, expr: &Expr) -> Option<&'static str> {
    let qualified = checker.semantic().qualified_name(expr)?;
    let dotted = match qualified.segments() {
        ["", builtin] => (*builtin).to_owned(),
        segments => segments.join("."),
    };
    lookup(ALIASES, &dotted).map(|(_, target)| *target).or_else(|| lookup(BASES, &dotted).map(|(name, _)| *name))
}

// The generator sorts both tables by name, so a lookup is a binary search.
fn lookup<T>(table: &'static [(&'static str, T)], name: &str) -> Option<&'static (&'static str, T)> {
    table.binary_search_by_key(&name, |(key, _)| key).ok().map(|index| &table[index])
}

pub(crate) fn derives_from_base_exception(lineage: &[ClassKey]) -> bool {
    lineage.contains(&ClassKey::Known("BaseException"))
}

// The class names `expr` derives from, itself first, as a docstring names them: by their last segment. A class the
// table and this file do not know is only its own name.
pub(crate) fn lineage_names<'a>(checker: &Checker<'a>, expr: &'a Expr) -> Option<Vec<&'a str>> {
    let Some(lineage) = lineage(checker, expr) else {
        return last_segment(expr).map(|name| vec![name]);
    };
    Some(
        lineage
            .into_iter()
            .filter_map(|key| match key {
                ClassKey::Known(name) => Some(last_dotted_segment(name)),
                ClassKey::Local(range) => checker
                    .class_defs()
                    .iter()
                    .copied()
                    .find(|class| class.range() == range)
                    .map(|class| class.name.as_str()),
            })
            .collect(),
    )
}

// The last segment of the class a written name stands for, so `IOError` and `socket.error` both read `OSError`.
pub(crate) fn canonical_name(name: &str) -> &str {
    last_dotted_segment(lookup(ALIASES, name).map_or(name, |(_, target)| *target))
}

// The classes an `except` clause names; `None` for a bare `except`, which catches every exception.
pub(crate) fn handled_types(handler: &ast::ExceptHandlerExceptHandler) -> Option<Vec<&Expr>> {
    Some(match handler.type_.as_deref()? {
        Expr::Tuple(tuple) => tuple.elts.iter().collect(),
        single => vec![single],
    })
}
