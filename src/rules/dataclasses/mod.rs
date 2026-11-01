mod dataclass_kw_only;
mod dataclass_options;

pub(crate) use dataclass_kw_only::dataclass_kw_only;
pub(crate) use dataclass_options::dataclass_options;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

fn require_options(
    checker: &Checker,
    rule: Rule,
    options: &[&str],
    applicability: impl Fn(&Checker, &ast::StmtClassDef, bool) -> Applicability,
) {
    // `slots=` and `kw_only=` arrived in Python 3.10; an older target cannot use them.
    if checker.settings.from_ruff.target_version < PythonVersion::PY310 {
        return;
    }
    let wanted: Vec<String> = options.iter().map(|option| format!("{option}=True")).collect();
    for class in checker.class_defs() {
        for decorator in &class.decorator_list {
            let (func, arguments) = match &decorator.expression {
                Expr::Call(ast::ExprCall { func, arguments, .. }) => (func.as_ref(), Some(arguments)),
                other => (other, None),
            };
            if !checker.semantic().resolves_to(func, &["dataclasses", "dataclass"]) {
                continue;
            }
            let keywords = arguments.map_or(&[][..], |arguments| &arguments.keywords);
            let missing: Vec<&str> = options
                .iter()
                .copied()
                .filter(|option| {
                    keywords
                        .iter()
                        .find(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == *option))
                        .is_none_or(|keyword| {
                            !keyword.value.as_boolean_literal_expr().is_some_and(|literal| literal.value)
                        })
                })
                .collect();
            if missing.is_empty() {
                continue;
            }
            let explicit =
                keywords.iter().any(|keyword| keyword.arg.as_ref().is_some_and(|arg| missing.contains(&arg.as_str())));
            let arguments: Vec<&str> = keywords
                .iter()
                .filter(|keyword| keyword.arg.as_ref().is_none_or(|arg| !options.contains(&arg.as_str())))
                .map(|keyword| checker.text(keyword.range()))
                .chain(wanted.iter().map(String::as_str))
                .collect();
            let range = decorator.expression.range();
            let fix = Fix::applicable_edit(
                Edit::range_replacement(format!("{}({})", checker.text(func), arguments.join(", ")), range),
                applicability(checker, class, explicit),
            );
            checker.report_fix(
                rule,
                range,
                format!("Dataclass is missing {}; set `{}`", missing.join(", "), wanted.join(", ")),
                Some(fix),
                format!("Pass `{}` to `@dataclass`", wanted.join(", ")),
            );
        }
    }
}
