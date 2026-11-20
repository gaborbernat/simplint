use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::rules::docstrings::is_documented_variable;
use crate::syntax::module_statements_with_next;
use crate::syntax::{annotation_name, instance_writes, is_classmethod, is_name, last_segment, target_names};

pub(crate) struct Attribute<'a> {
    pub(crate) name: &'a str,
    // A class default also assigned on the instance can have both flags.
    pub(crate) class_var: bool,
    pub(crate) on_instance: bool,
    pub(crate) documented: bool,
}

pub(super) fn attributes<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> Vec<Attribute<'a>> {
    let mut found: Vec<Attribute> = Vec::new();
    let mut index: FxHashMap<&'a str, usize> = FxHashMap::default();
    let mut add = |name: &'a str, class_var: bool, documented: bool| {
        if let Some(&position) = index.get(name) {
            let attribute = &mut found[position];
            attribute.documented |= documented;
            attribute.on_instance |= !class_var;
        } else {
            index.insert(name, found.len());
            found.push(Attribute { name, class_var, on_instance: !class_var, documented });
        }
    };
    for (stmt, next) in module_statements_with_next(&class.body) {
        let documented = is_documented_variable(checker, stmt, next);
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) => {
                // For an ordinary class, PEP 591 treats an initialized Final in the class body as a class variable.
                let qualifier = annotation_name(annotation);
                let class_var = (value.is_some() && qualifier == Some("Final")) || qualifier == Some("ClassVar");
                for name in std::iter::once(target.as_ref()).filter_map(Expr::as_name_expr) {
                    add(name.id.as_str(), class_var, documented);
                }
                if let Some(value) = value.as_deref().filter(|_| is_name(target, "__slots__")) {
                    for name in slot_names(value) {
                        add(name, false, false);
                    }
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                // An attrs field can be declared without an annotation.
                for name in targets.iter().flat_map(target_names) {
                    add(name, !is_field_call(value), documented);
                }
                // Declaring a slot supplies a name but no description.
                if targets.iter().any(|target| is_name(target, "__slots__")) {
                    for name in slot_names(value) {
                        add(name, false, false);
                    }
                }
            }
            Stmt::TypeAlias(ast::StmtTypeAlias { name, .. }) => {
                for name in target_names(name) {
                    add(name, true, documented);
                }
            }
            Stmt::For(ast::StmtFor { target, .. }) => {
                for name in target_names(target) {
                    add(name, true, documented);
                }
            }
            _ => {}
        }
    }
    for write in instance_writes(class) {
        // Deleting an attribute does not establish that this class created it.
        if matches!(write.stmt, Stmt::Delete(_)) {
            continue;
        }
        add(write.attr, is_classmethod(write.method), is_documented_variable(checker, write.stmt, write.next));
    }
    found
}

fn slot_names(value: &Expr) -> Vec<&str> {
    let names: Vec<&Expr> = match value {
        Expr::Tuple(ast::ExprTuple { elts, .. })
        | Expr::List(ast::ExprList { elts, .. })
        | Expr::Set(ast::ExprSet { elts, .. }) => elts.iter().collect(),
        Expr::Dict(ast::ExprDict { items, .. }) => items.iter().filter_map(|item| item.key.as_ref()).collect(),
        _ => vec![value],
    };
    names.into_iter().filter_map(|name| name.as_string_literal_expr().map(|literal| literal.value.to_str())).collect()
}

pub(super) fn is_field_call(value: &Expr) -> bool {
    value.as_call_expr().is_some_and(|call| matches!(last_segment(&call.func), Some("ib" | "attrib" | "field")))
}

// Unknown bases may supply attributes or constructor parameters.
pub(crate) fn has_attribute_bases(class: &ast::StmtClassDef) -> bool {
    class.bases().iter().any(|base| {
        !matches!(
            last_segment(map_subscript(base)),
            Some("object" | "ABC" | "Protocol" | "Generic" | "NamedTuple" | "TypedDict")
        )
    })
}

pub(super) fn skips_init(call: &ast::ExprCall) -> bool {
    call.arguments
        .find_keyword("init")
        .is_some_and(|keyword| keyword.value.as_boolean_literal_expr().is_some_and(|literal| !literal.value))
}
