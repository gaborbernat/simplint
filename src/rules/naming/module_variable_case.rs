use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::module_variable::module_variables;
use crate::rules::rename::rename_fix;

pub fn module_variable_case(checker: &Checker) {
    for variable in module_variables(checker) {
        let stem = variable.name.trim_start_matches('_');
        let prefix = &variable.name[..variable.name.len() - stem.len()];
        let expected = format!("{prefix}{}", stem.to_ascii_uppercase());
        if expected == variable.name {
            continue;
        }
        let fix = rename_fix(checker, variable.name, &expected);
        checker.report_fix(
            Rule::ModuleVariableCase,
            variable.range,
            format!("Module-scope variable `{}` should be named `{expected}`", variable.name),
            fix,
            format!("Rename `{}` to `{expected}`", variable.name),
        );
    }
}
