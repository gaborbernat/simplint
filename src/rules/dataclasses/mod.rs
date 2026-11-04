mod dataclass_kw_only;
mod dataclass_options;

pub(crate) use dataclass_kw_only::dataclass_kw_only;
pub(crate) use dataclass_options::dataclass_options;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

// A dataclass decorator that leaves some of a rule's options unset.
struct Decorated<'a> {
    class: &'a ast::StmtClassDef,
    missing: Vec<&'static str>,
    // Whether the decorator sets one of the missing options to something other than `True`.
    explicit: bool,
}

// The options a fix sets, any edits it makes beyond the decorator, and how safe it is.
struct Offer {
    options: Vec<&'static str>,
    edits: Vec<Edit>,
    applicability: Applicability,
}

fn require_options(
    checker: &Checker,
    rule: Rule,
    options: &[&'static str],
    offer: impl Fn(&Decorated) -> Option<Offer>,
) {
    // `slots=` and `kw_only=` arrived in Python 3.10; an older target cannot use them.
    if checker.settings.from_ruff.target_version < PythonVersion::PY310 {
        return;
    }
    let wanted = settings(options);
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
            let range = decorator.expression.range();
            let offered = offer(&Decorated { class, missing: missing.clone(), explicit })
                .filter(|offer| !offer.options.is_empty());
            let title = offered
                .as_ref()
                .map_or_else(String::new, |offer| format!("Pass `{}` to `@dataclass`", settings(&offer.options)));
            let fix = offered.map(|Offer { options, edits, applicability }| {
                let arguments: Vec<&str> = keywords
                    .iter()
                    .filter(|keyword| keyword.arg.as_ref().is_none_or(|arg| !options.contains(&arg.as_str())))
                    .map(|keyword| checker.text(keyword.range()))
                    .collect();
                let arguments =
                    arguments.into_iter().map(str::to_owned).chain([settings(&options)]).collect::<Vec<_>>();
                let decorator =
                    Edit::range_replacement(format!("{}({})", checker.text(func), arguments.join(", ")), range);
                Fix::applicable_edits(decorator, edits, applicability)
            });
            checker.report_fix(
                rule,
                range,
                format!("Dataclass is missing {}; set `{wanted}`", missing.join(", ")),
                fix,
                title,
            );
        }
    }
}

// `frozen=True, slots=True` for `["frozen", "slots"]`.
fn settings(options: &[&str]) -> String {
    options.iter().map(|option| format!("{option}=True")).collect::<Vec<_>>().join(", ")
}
