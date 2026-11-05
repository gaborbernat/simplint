use ruff_python_ast::{self as ast, Expr, Stmt};

use crate::lint::checker::Checker;
use crate::rules::docstrings::{is_documented_variable, module_statements_with_next};
use crate::syntax::{instance_writes, is_classmethod, last_segment};

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

pub(crate) fn attributes<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> Vec<Attribute<'a>> {
    let mut found: Vec<Attribute> = Vec::new();
    let mut add = |name: &'a str, class_var: bool, documented: bool| match found
        .iter_mut()
        .find(|attribute| attribute.name == name)
    {
        Some(attribute) => {
            attribute.documented |= documented;
            attribute.on_instance |= !class_var;
        }
        None => found.push(Attribute { name, class_var, on_instance: !class_var, documented }),
    };
    for (stmt, next) in module_statements_with_next(&class.body) {
        let documented = is_documented_variable(checker, stmt, next);
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) => {
                if let Some(name) = target.as_name_expr() {
                    add(name.id.as_str(), last_segment(unsubscripted(annotation)) == Some("ClassVar"), documented);
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                // An attrs or dataclass field declared by a call holds a value per instance.
                for name in targets.iter().filter_map(Expr::as_name_expr) {
                    add(name.id.as_str(), !is_field_call(value), documented);
                }
            }
            Stmt::TypeAlias(ast::StmtTypeAlias { name, .. }) => {
                if let Some(name) = name.as_name_expr() {
                    add(name.id.as_str(), true, documented);
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

// Whether the class takes attributes from a base that may define them, as a field list cannot tell.
pub(crate) fn has_attribute_bases(class: &ast::StmtClassDef) -> bool {
    class.bases().iter().any(|base| {
        !matches!(
            last_segment(unsubscripted(base)),
            Some("object" | "ABC" | "Protocol" | "Generic" | "NamedTuple" | "TypedDict")
        )
    })
}

// An annotation or base without its parameters, as `ClassVar` for `ClassVar[int]`.
pub(super) fn unsubscripted(expr: &Expr) -> &Expr {
    expr.as_subscript_expr().map_or(expr, |subscript| &subscript.value)
}

// Whether the value declares an attrs or dataclass field, which holds a value per instance.
pub(super) fn is_field_call(value: &Expr) -> bool {
    value.as_call_expr().is_some_and(|call| matches!(last_segment(&call.func), Some("ib" | "attrib" | "field")))
}

// Whether a class generator or field call passes `init=False`, leaving the field or the whole class out of the
// generated `__init__`.
pub(super) fn skips_init(call: &ast::ExprCall) -> bool {
    call.arguments
        .find_keyword("init")
        .is_some_and(|keyword| keyword.value.as_boolean_literal_expr().is_some_and(|literal| !literal.value))
}
