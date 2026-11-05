mod dataclass_frozen;
mod dataclass_hazards;
pub(crate) mod dataclass_kw_only;
mod dataclass_slots;
mod indistinct_error_class;

pub(crate) use dataclass_frozen::dataclass_frozen;
pub(crate) use dataclass_kw_only::dataclass_kw_only;
pub(crate) use dataclass_slots::dataclass_slots;
pub(crate) use indistinct_error_class::indistinct_error_class;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::{self as ast, Expr};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{dataclass_decorator, keyword_is_true};

// A dataclass decorator that leaves some of a rule's options unset.
struct Decorated<'a> {
    class: &'a ast::StmtClassDef,
    missing: Vec<&'static str>,
    keywords: &'a [ast::Keyword],
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
    // `slots=` and `kw_only=` arrived in Python 3.10; an older target cannot use them. `frozen=` has no such limit.
    if options.iter().any(|option| matches!(*option, "slots" | "kw_only"))
        && checker.settings.from_ruff.target_version < dataclass_kw_only::SINCE
    {
        return;
    }
    let wanted = settings(options);
    for class in checker.class_defs() {
        let is_dataclass = |func: &Expr| checker.semantic().resolves_to(func, &["dataclasses", "dataclass"]);
        let Some(decorator) = dataclass_decorator(class, &is_dataclass) else {
            continue;
        };
        let (func, keywords) = match decorator {
            Expr::Call(ast::ExprCall { func, arguments, .. }) => (func.as_ref(), &*arguments.keywords),
            other => (other, &[][..]),
        };
        let missing: Vec<&str> = options.iter().copied().filter(|option| !keyword_is_true(decorator, option)).collect();
        if missing.is_empty() {
            continue;
        }
        let range = decorator.range();
        let offered =
            offer(&Decorated { class, missing: missing.clone(), keywords }).filter(|offer| !offer.options.is_empty());
        let title = offered
            .as_ref()
            .map_or_else(String::new, |offer| format!("Pass `{}` to `@dataclass`", settings(&offer.options)));
        let fix = offered.map(|Offer { options, edits, applicability }| {
            let arguments = decorator_arguments(checker, keywords, &options);
            let edit = Edit::range_replacement(format!("{}({})", checker.text(func), arguments.join(", ")), range);
            Fix::applicable_edits(edit, edits, applicability)
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

// `frozen=True, slots=True` for `["frozen", "slots"]`.
fn settings(options: &[&str]) -> String {
    options.iter().map(|option| format!("{option}=True")).collect::<Vec<_>>().join(", ")
}

// The decorator's arguments with `options` set, placed before a `slots` the decorator lists when `frozen` is added, so
// `frozen=True` lands before `slots=True` whichever of the two fixes runs first.
fn decorator_arguments(checker: &Checker, keywords: &[ast::Keyword], options: &[&str]) -> Vec<String> {
    let kept: Vec<&ast::Keyword> = keywords
        .iter()
        .filter(|keyword| !keyword.arg.as_ref().is_some_and(|arg| options.contains(&arg.as_str())))
        .collect();
    let first = options.iter().filter_map(|option| option_rank(option)).min().unwrap_or(usize::MAX);
    let at = kept
        .iter()
        .position(|keyword| {
            keyword.arg.as_ref().and_then(|arg| option_rank(arg.as_str())).is_some_and(|rank| rank > first)
        })
        .unwrap_or(kept.len());
    let mut arguments: Vec<String> = kept.iter().map(|keyword| checker.text(keyword.range()).to_owned()).collect();
    arguments.insert(at, settings(options));
    arguments
}

// The order in which `frozen` precedes `slots`; the other options are appended.
fn option_rank(option: &str) -> Option<usize> {
    ["frozen", "slots"].iter().position(|name| *name == option)
}
