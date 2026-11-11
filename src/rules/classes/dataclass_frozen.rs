use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::{self as ast, Stmt};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::{analyze, field_keyword};
use crate::rules::classes::{Offer, require_options};
use crate::syntax::Visibility;

pub fn dataclass_frozen(checker: &Checker) {
    let stub = checker.path.extension().is_some_and(|extension| extension == "pyi");
    require_options(checker, Rule::DataclassFrozen, &["frozen"], |decorated, missing| {
        let analysis = analyze(checker, decorated)?;
        let options = if analysis.frozen_breaks { Vec::new() } else { missing.to_vec() };
        let from_ruff = &checker.settings.from_ruff;
        let plain_form_rejected = (from_ruff.ruff_runs(RuffRule::ClassAsDataStructure)
            && is_data_structure_shape(decorated.class))
            || (from_ruff.ruff_runs(RuffRule::TooManyArguments) && analysis.parameters > from_ruff.max_args);
        Some(Offer {
            left_to_ruff: !stub && analysis.mutated && plain_form_rejected,
            ..Offer::options(options, analysis.applicability())
        })
    });
}

// The shape B903 reports: no base classes or class keywords, no decorator besides the dataclass's, and a body whose
// methods are all private. Each field must be a plain parameter: a `field(...)` with a factory or `init` has no such
// default, and a hand-written `__init__` is not one of the dataclass's own.
fn is_data_structure_shape(class: &ast::StmtClassDef) -> bool {
    class.arguments.is_none()
        && class.decorator_list.len() == 1
        && class.body.iter().all(|stmt| match stmt {
            Stmt::FunctionDef(function) => Visibility::of(function.name.as_str()) == Visibility::Private,
            Stmt::AnnAssign(assign) => {
                field_keyword(assign.value.as_deref(), "default_factory").is_none()
                    && field_keyword(assign.value.as_deref(), "init").is_none()
            }
            Stmt::Assign(_) | Stmt::Expr(_) => true,
            _ => false,
        })
}
