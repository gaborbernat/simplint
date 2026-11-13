use ruff_python_ast::helpers::map_subscript;
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::rules::docstrings::is_documented_variable;
use crate::syntax::module_statements_with_next;
use crate::syntax::{annotation_name, instance_writes, is_classmethod, is_name, last_segment, target_names};

// An attribute of a class as autodoc documents it: one its body assigns, or one a method sets on `self`.
pub(crate) struct Attribute<'a> {
    pub(crate) name: &'a str,
    // Whether it belongs to the class rather than its instances: a `ClassVar`, or a name the body assigns without an
    // annotation; and whether instances hold it, as an annotated field or a name a method sets on `self`, which a class
    // default may be too.
    pub(crate) class_var: bool,
    pub(crate) on_instance: bool,
    // Whether a `#:` comment or a string right after it documents it, as autodoc reads.
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
                // A `Final` with a value in a class body is a class variable, as PEP 591 says; without one it is an
                // instance attribute set in `__init__`.
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
                // An attrs or dataclass field declared by a call holds a value per instance.
                for name in targets.iter().flat_map(target_names) {
                    add(name, !is_field_call(value), documented);
                }
                // A slot is an instance attribute, which the body does not document by itself.
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
            // A loop variable of the class body is a class variable, as an assignment to it is.
            Stmt::For(ast::StmtFor { target, .. }) => {
                for name in target_names(target) {
                    add(name, true, documented);
                }
            }
            _ => {}
        }
    }
    for write in instance_writes(class) {
        // A `del` leaves the attribute undocumented, as the body does not set it.
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

// Whether the class takes attributes from a base that may define them, as a field list cannot tell.
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
