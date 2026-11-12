use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{dataclass_decorator, decorator_names, instance_writes, last_segment};

// A class holding many attributes mixes several responsibilities, which smaller classes it holds would each name.
pub fn too_many_attributes(checker: &Checker) {
    let limit = checker.settings.project.knobs.max_instance_attributes;
    for class in checker.class_defs() {
        let mut attributes: FxHashSet<&str> = instance_writes(class)
            .into_iter()
            .filter(|write| !decorator_names(&write.method.decorator_list).contains(&"classmethod"))
            .map(|write| write.attr)
            .collect();
        if is_dataclass(checker, class) {
            attributes.extend(fields(class));
        }
        let count = attributes.len();
        if count > limit {
            checker.report(
                Rule::TooManyAttributes,
                class.name.range(),
                format!(
                    "`{}` holds {count} instance attributes, over the limit of {limit}; group related ones into \
                     classes of their own",
                    class.name
                ),
            );
        }
    }
}

fn is_dataclass(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    dataclass_decorator(class, &|func| checker.semantic().resolves_to(func, &["dataclasses", "dataclass"])).is_some()
}

// The fields a dataclass's annotated class attributes declare, leaving out `ClassVar` ones and the `KW_ONLY` marker.
fn fields(class: &ast::StmtClassDef) -> impl Iterator<Item = &str> {
    class.body.iter().filter_map(|stmt| {
        let Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, .. }) = stmt else {
            return None;
        };
        let base = match annotation.as_ref() {
            Expr::Subscript(subscript) => subscript.value.as_ref(),
            other => other,
        };
        if matches!(last_segment(base), Some("ClassVar" | "KW_ONLY")) {
            return None;
        }
        Some(target.as_name_expr()?.id.as_str())
    })
}
