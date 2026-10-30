use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::docstring;

pub fn forwarding_function(checker: &mut Checker) {
    let mut found = Vec::new();
    for stmt in checker.body() {
        match stmt {
            Stmt::FunctionDef(function) => {
                let calls: Vec<TextRange> =
                    checker.semantic().references_to(function.name.as_str()).map(|reference| reference.range).collect();
                found.extend(forwarded(checker, function, &calls));
            }
            Stmt::ClassDef(class) if class.arguments.as_ref().is_none_or(|arguments| arguments.args.is_empty()) => {
                for function in class.body.iter().filter_map(Stmt::as_function_def_stmt) {
                    let calls: Vec<TextRange> = checker
                        .semantic()
                        .member_references
                        .iter()
                        .filter(|reference| reference.class == class.range && reference.name == function.name.as_str())
                        .map(|reference| reference.range)
                        .collect();
                    found.extend(forwarded(checker, function, &calls));
                }
            }
            _ => {}
        }
    }
    for (range, name, target) in found {
        checker.report(
            Rule::ForwardingFunction,
            range,
            format!("`{name}` only forwards to `{target}`; call `{target}` at its call sites instead"),
            None,
        );
    }
}

fn forwarded<'a>(
    checker: &Checker<'a>,
    function: &'a ast::StmtFunctionDef,
    references: &[TextRange],
) -> Option<(TextRange, &'a str, &'a str)> {
    let name = function.name.as_str();
    if !name.starts_with('_') || is_dunder(name) || !function.decorator_list.is_empty() || references.is_empty() {
        return None;
    }
    let [statement] = &function.body[usize::from(docstring(&function.body).is_some())..] else {
        return None;
    };
    let call = match statement {
        Stmt::Return(ast::StmtReturn { value: Some(value), .. }) | Stmt::Expr(ast::StmtExpr { value, .. }) => {
            value.as_call_expr()?
        }
        _ => return None,
    };
    references
        .iter()
        .all(|range| checker.source[range.end().to_usize()..].trim_start().starts_with('('))
        .then(|| (function.name.range(), name, checker.text(call.func.range())))
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::ForwardingFunction, "SMP030.py" ; "forwarding_function")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
