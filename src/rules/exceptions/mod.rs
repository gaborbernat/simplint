mod hierarchy;

use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;

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

pub(crate) fn derives_from_base_exception(lineage: &[ClassKey]) -> bool {
    lineage.contains(&ClassKey::Known("BaseException"))
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
        pending.extend(bases_of(name).iter().rev().copied());
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
    ALIASES
        .iter()
        .find(|(alias, _)| *alias == dotted)
        .map(|(_, target)| *target)
        .or_else(|| BASES.iter().find(|(name, _)| *name == dotted).map(|(name, _)| *name))
}

fn bases_of(name: &str) -> &'static [&'static str] {
    BASES.iter().find(|(class, _)| *class == name).map_or(&[], |(_, bases)| bases)
}
