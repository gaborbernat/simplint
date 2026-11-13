use ruff_python_ast as ast;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{Declares, dataclass_decorator, dataclass_fields, instance_writes, is_classmethod};

// A class holding many attributes mixes several responsibilities, which smaller classes it holds would each name.
pub fn too_many_attributes(checker: &Checker) {
    let limit = checker.settings.project.knobs.max_instance_attributes;
    for class in checker.class_defs() {
        let mut attributes: FxHashSet<&str> = instance_writes(class)
            .into_iter()
            .filter(|write| !is_classmethod(write.method))
            .map(|write| write.attr)
            .collect();
        if is_dataclass(checker, class) {
            attributes.extend(
                dataclass_fields(&class.body).filter(|field| field.kind == Declares::Field).map(|field| field.name),
            );
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
