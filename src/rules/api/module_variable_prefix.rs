use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::module_variable::module_variables;
use crate::rules::rename::rename_fix;

pub fn module_variable_prefix(checker: &Checker) {
    for variable in module_variables(checker) {
        if variable.name.starts_with('_') == variable.private {
            continue;
        }
        let (expected, message) = if variable.private {
            let expected = format!("_{}", variable.name);
            let message = format!("`{}` is not in `__all__`; name it `{expected}`", variable.name);
            (expected, message)
        } else {
            let expected = variable.name.trim_start_matches('_').to_owned();
            let message = format!("`{}` is in `__all__`; name it `{expected}`", variable.name);
            (expected, message)
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
