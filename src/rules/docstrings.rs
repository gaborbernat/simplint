use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::Stmt;
use ruff_text_size::Ranged;

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{class_defs, docstring, function_defs, is_alone_on_lines, lines_with_trailing_blanks};

pub fn function_docstring(checker: &mut Checker) {
    let body = checker.body();
    let found: Vec<(&[Stmt], &str)> = function_defs(body)
        .into_iter()
        .map(|function| (function.body.as_slice(), "Function"))
        .chain(class_defs(body).into_iter().map(|class| (class.body.as_slice(), "Class")))
        .filter(|(owner_body, _)| docstring(owner_body).is_some())
        .collect();
    for (owner_body, kind) in found {
        let stmt = &owner_body[0];
        let fix = is_alone_on_lines(checker.source, stmt.range()).then(|| {
            let edit = if owner_body.len() == 1 {
                Edit::range_replacement("...".to_owned(), stmt.range())
            } else {
                Edit::range_deletion(lines_with_trailing_blanks(checker.source, stmt.range()))
            };
            Fix::unsafe_edit(edit)
        });
        checker.report(
            Rule::FunctionDocstring,
            stmt.range(),
            format!("{kind} docstring; let the name, signature and types document it"),
            fix,
        );
    }
}

pub fn multiline_module_docstring(checker: &mut Checker) {
    if checker.is_test_module() {
        return;
    }
    if let Some(literal) = docstring(checker.body())
        && literal.value.to_str().trim().contains('\n')
    {
        checker.report(
            Rule::MultilineModuleDocstring,
            literal.range(),
            "Module docstring spans more than one line; keep one line saying what the module is",
            None,
        );
    }
}

pub fn test_module_docstring(checker: &mut Checker) {
    if !checker.is_test_module() {
        return;
    }
    let body = checker.body();
    let Some(literal) = docstring(body) else {
        return;
    };
    let stmt = &body[0];
    let fix = is_alone_on_lines(checker.source, stmt.range())
        .then(|| Fix::safe_edit(Edit::range_deletion(lines_with_trailing_blanks(checker.source, stmt.range()))));
    checker.report(Rule::TestModuleDocstring, literal.range(), "Test module carries a docstring; remove it", fix);
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::FunctionDocstring, "SMP002.py" ; "function_docstring")]
    #[test_case(Rule::MultilineModuleDocstring, "SMP003.py" ; "multiline_module_docstring")]
    #[test_case(Rule::MultilineModuleDocstring, "SMP003_ok.py" ; "multiline_module_docstring_single_line")]
    #[test_case(Rule::MultilineModuleDocstring, "tests/test_SMP004.py" ; "multiline_module_docstring_test_module")]
    #[test_case(Rule::TestModuleDocstring, "tests/test_SMP004.py" ; "test_module_docstring")]
    #[test_case(Rule::TestModuleDocstring, "SMP003.py" ; "test_module_docstring_non_test")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
