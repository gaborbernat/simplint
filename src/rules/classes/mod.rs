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
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::{Checker, Related};
use crate::lint::registry::Rule;
use crate::syntax::{ExprCollector, Visibility, dataclass_decorator, is_name, is_true_literal};

// The classes a library's users construct and subclass: its public classes and the public classes nested in them, as
// the options a rule adds change what those users can do with an instance. A public name a module with `__all__` leaves
// out is not one, unless a public name's signature refers to it.
fn library_api<'a>(checker: &'a Checker) -> impl Fn(&ast::StmtClassDef) -> bool + 'a {
    let api: Vec<TextRange> = checker
        .body()
        .iter()
        .filter_map(Stmt::as_class_def_stmt)
        .filter(|class| checker.project().is_library_api(class.name.as_str()))
        .map(Ranged::range)
        .collect();
    move |class| {
        Visibility::of(class.name.as_str()) == Visibility::Public
            && api.iter().any(|outer| outer.contains_range(class.range()))
    }
}

// A dataclass decorator, with the keywords it passes and the options its `**` splats pass.
struct Decorated<'a> {
    class: &'a ast::StmtClassDef,
    keywords: &'a [ast::Keyword],
    splat: &'a [(&'a str, &'a Expr)],
    // A decorator below `@dataclass` runs first, and `slots=True` makes the dataclass return a new class, which the
    // decorator below never saw.
    wrapped: bool,
}

impl<'a> Decorated<'a> {
    // The value the decorator passes for `name`, by keyword or by a splat. A splat cannot repeat a keyword's name.
    fn option(&self, name: &str) -> Option<&'a Expr> {
        self.keywords
            .iter()
            .find(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == name))
            .map(|keyword| &keyword.value)
            .or_else(|| splat_value(self.splat, name))
    }
}

// The options a fix sets, any edits it makes beyond the decorator, and how safe it is. With no options the report has no
// fix; it still points at `related` and adds `note` to its message. A `left_to_ruff` offer reports nothing.
struct Offer {
    options: Vec<&'static str>,
    edits: Vec<Edit>,
    applicability: Applicability,
    related: Vec<Related>,
    note: String,
    left_to_ruff: bool,
}

fn require_options(
    checker: &Checker,
    rule: Rule,
    options: &[&'static str],
    offer: impl Fn(&Decorated, &[&'static str]) -> Option<Offer>,
) {
    // `slots=` and `kw_only=` arrived in Python 3.10; an older target cannot use them. `frozen=` has no such limit.
    if options.iter().any(|option| matches!(*option, "slots" | "kw_only"))
        && checker.target_version() < dataclass_kw_only::SINCE
    {
        return;
    }
    let wanted = settings(options);
    // A library's public dataclasses are left alone, since their users construct, subclass or call them by position,
    // which these options change.
    let exempt = library_api(checker);
    for class in checker.class_defs() {
        let is_dataclass = |func: &Expr| checker.semantic().resolves_to(func, &["dataclasses", "dataclass"]);
        let Some(decorator) = dataclass_decorator(class, &is_dataclass) else {
            continue;
        };
        if exempt(class) {
            continue;
        }
        let (func, keywords) = match decorator {
            Expr::Call(ast::ExprCall { func, arguments, .. }) => (func.as_ref(), &*arguments.keywords),
            other => (other, &[][..]),
        };
        // A splat whose keys cannot be read may set any option, so the class is left alone.
        let Some(splat) = splatted_options(checker, keywords) else {
            continue;
        };
        let decorated = Decorated {
            class,
            keywords,
            splat: &splat,
            wrapped: class.decorator_list.iter().any(|other| other.start() > decorator.end()),
        };
        let missing: Vec<&'static str> =
            options.iter().copied().filter(|option| !decorated.option(option).is_some_and(is_true_literal)).collect();
        if missing.is_empty() {
            continue;
        }
        let range = decorator.range();
        // The fix cannot take a key out of a splat, so adding the option would repeat it. The offer still runs for
        // its related locations and note.
        let passed = missing.iter().any(|option| splat_value(&splat, option).is_some());
        let offer = offer(&decorated, &missing);
        if offer.as_ref().is_some_and(|offer| offer.left_to_ruff) {
            continue;
        }
        let offered = offer.map(|offer| Offer { options: if passed { Vec::new() } else { offer.options }, ..offer });
        let (fix, title, related, note) = match offered {
            // Rewriting the decorator would drop the comments inside it, so a commented decorator gets no fix.
            Some(Offer { options, edits, applicability, related, note, .. })
                if !options.is_empty() && !checker.has_comment_in(range) =>
            {
                let arguments = decorator_arguments(checker, keywords, &options);
                let edit = Edit::range_replacement(format!("{}({})", checker.text(func), arguments.join(", ")), range);
                let fix = Fix::applicable_edits(edit, edits, applicability);
                (Some(fix), format!("Pass `{}` to `@dataclass`", settings(&options)), related, note)
            }
            Some(Offer { related, note, .. }) => (None, String::new(), related, note),
            None => (None, String::new(), Vec::new(), String::new()),
        };
        checker.report_related(
            rule,
            range,
            format!("Dataclass is missing {}; set `{wanted}`{note}", missing.join(", ")),
            fix,
            title,
            related,
        );
    }
}

// The options the `**` arguments among `keywords` pass with their values, or `None` when one of them cannot be read.
pub(crate) fn splatted_options<'s>(
    checker: &'s Checker,
    keywords: &'s [ast::Keyword],
) -> Option<Vec<(&'s str, &'s Expr)>> {
    let mut passed = Vec::new();
    for keyword in keywords.iter().filter(|keyword| keyword.arg.is_none()) {
        passed.extend(splat_pairs(checker, &keyword.value)?);
    }
    // A key repeated across the splats is a runtime error, so no fix may rely on its value.
    let mut seen = FxHashSet::default();
    passed.iter().all(|(option, _)| seen.insert(*option)).then_some(passed)
}

// The name's value when the splat is a module-level name bound once, to a dict literal or `dict(...)` call.
fn splat_pairs<'s>(checker: &'s Checker, value: &'s Expr) -> Option<Vec<(&'s str, &'s Expr)>> {
    let Expr::Name(name) = value else {
        return literal_pairs(checker, value);
    };
    let id = name.id.as_str();
    // A later store or dict method changes the keys the literal gives, which nothing here follows.
    if checker.semantic().bindings(id).len() != 1 || is_mutated(checker, id) {
        return None;
    }
    literal_pairs(checker, checker.semantic().only_binding_value(name)?)
}

fn literal_pairs<'s>(checker: &'s Checker, value: &'s Expr) -> Option<Vec<(&'s str, &'s Expr)>> {
    match value {
        Expr::Dict(ast::ExprDict { items, .. }) => items
            .iter()
            .map(|item| Some((item.key.as_ref()?.as_string_literal_expr()?.value.to_str(), &item.value)))
            .collect(),
        Expr::Call(ast::ExprCall { func, arguments, .. })
            if arguments.args.is_empty() && checker.semantic().is_builtin(func, "dict") =>
        {
            arguments.keywords.iter().map(|keyword| Some((keyword.arg.as_ref()?.as_str(), &keyword.value))).collect()
        }
        _ => None,
    }
}

// Whether the module writes into the dict `name`, by item assignment, deletion or a method that changes it.
fn is_mutated(checker: &Checker, name: &str) -> bool {
    const MUTATORS: [&str; 5] = ["update", "pop", "popitem", "setdefault", "clear"];
    let mut writes = ExprCollector::new(|expr: &Expr| {
        let written = match expr {
            Expr::Subscript(ast::ExprSubscript { value, ctx, .. }) => {
                matches!(ctx, ExprContext::Store | ExprContext::Del) && is_name(value, name)
            }
            Expr::Call(ast::ExprCall { func, .. }) => func.as_attribute_expr().is_some_and(|attribute| {
                is_name(&attribute.value, name) && MUTATORS.contains(&attribute.attr.as_str())
            }),
            _ => false,
        };
        written.then_some(())
    });
    writes.visit_body(checker.body());
    !writes.found.is_empty()
}

fn splat_value<'a>(splat: &[(&str, &'a Expr)], option: &str) -> Option<&'a Expr> {
    splat.iter().find(|(name, _)| *name == option).map(|(_, value)| *value)
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
