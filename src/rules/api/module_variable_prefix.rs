use ruff_python_stdlib::identifiers::is_identifier;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::rename::{kept_message, rename_fix};

pub fn module_variable_prefix(checker: &Checker) {
    let public = checker.project().role().public;
    for variable in checker.state().module_variables(checker) {
        if variable.name.starts_with('_') == variable.private {
            continue;
        }
        let expected = if variable.private {
            format!("_{}", variable.name)
        } else {
            variable.name.trim_start_matches('_').to_owned()
        };
        // A name with no valid spelling, as `_` in `__all__`, has nothing to rename it to.
        if !is_identifier(&expected) {
            continue;
        }
        // A public module's variables are the API its users import, so the author lists or renames them by hand.
        if public {
            checker.report(
                Rule::ModuleVariablePrefix,
                variable.range,
                kept_message(variable.name, &expected, !variable.private),
            );
            continue;
        }
        let message = if variable.private {
            format!("`{}` is not in `__all__`; name it `{expected}`", variable.name)
        } else {
            format!("`{}` is in `__all__`; name it `{expected}`", variable.name)
        };
        let fix = rename_fix(checker, variable.name, &expected);
        checker.report_fix(
            Rule::ModuleVariablePrefix,
            variable.range,
            message,
            fix,
            format!("Rename `{}` to `{expected}`", variable.name),
        );
    }
}
