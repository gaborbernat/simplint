use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::class_defs;

pub fn dataclass_options(checker: &mut Checker) {
    const OPTIONS: [&str; 3] = ["frozen", "slots", "kw_only"];
    let mut found = Vec::new();
    for class in class_defs(checker.body()) {
        for decorator in &class.decorator_list {
            let (func, arguments) = match &decorator.expression {
                Expr::Call(ast::ExprCall { func, arguments, .. }) => (func.as_ref(), Some(arguments)),
                other => (other, None),
            };
            if !UnqualifiedName::from_expr(func)
                .is_some_and(|name| matches!(name.segments(), ["dataclass"] | ["dataclasses", "dataclass"]))
            {
                continue;
            }
            let missing: Vec<&str> = OPTIONS
                .into_iter()
                .filter(|option| {
                    arguments.is_none_or(|arguments| {
                        arguments.find_keyword(option).is_none_or(|keyword| {
                            !keyword.value.as_boolean_literal_expr().is_some_and(|literal| literal.value)
                        })
                    })
                })
                .collect();
            if !missing.is_empty() {
                found.push((decorator.expression.range(), func.range(), arguments, missing));
            }
        }
    }
    for (range, func, arguments, missing) in found {
        let options: Vec<String> = arguments
            .into_iter()
            .flat_map(|arguments| &arguments.keywords)
            .filter(|keyword| keyword.arg.as_ref().is_none_or(|arg| !OPTIONS.contains(&arg.as_str())))
            .map(|keyword| checker.text(keyword.range()).to_owned())
            .chain(OPTIONS.iter().map(|option| format!("{option}=True")))
            .collect();
        checker.report(
            Rule::DataclassOptions,
            range,
            format!("Dataclass is missing {}; set `frozen=True, slots=True, kw_only=True`", missing.join(", ")),
            Some(Fix::unsafe_edit(Edit::range_replacement(
                format!("{}({})", checker.text(func), options.join(", ")),
                range,
            ))),
        );
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::DataclassOptions, "SMP111.py" ; "dataclass_options")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
