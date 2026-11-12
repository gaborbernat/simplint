pub(crate) mod async_sync_dunder;
pub(crate) mod broken_decorator_stack;
mod dataclass_frozen;
mod dataclass_hazards;
mod dataclass_kw_only;
mod dataclass_slots;
pub(crate) mod exception_init_args;
mod indistinct_error_class;
pub(crate) mod rejected_operand;
pub(crate) mod unbound_super;
mod too_many_attributes;

pub(crate) use dataclass_frozen::dataclass_frozen;
pub(crate) use dataclass_hazards::expected_to_raise_classes;
pub(crate) use dataclass_kw_only::dataclass_kw_only;
pub(crate) use dataclass_slots::dataclass_slots;
pub(crate) use indistinct_error_class::indistinct_error_class;
pub(crate) use too_many_attributes::too_many_attributes;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::{Checker, Related};
use crate::lint::registry::Rule;
use crate::syntax::{ExprCollector, Visibility, dataclass_decorator, is_name, is_true_literal};

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
    // A library's public dataclasses are left alone, since their users construct, subclass or call them by position,
    // which these options change.
    let exempt = library_api(checker);
    for class in checker.class_defs() {
        let Some(decorated) = Decorated::parse(checker, class) else {
            continue;
        };
        if exempt(class) {
            continue;
        }
        let missing: Vec<&'static str> =
            options.iter().copied().filter(|option| !decorated.option(option).is_some_and(is_true_literal)).collect();
        if missing.is_empty() {
            continue;
        }
        let range = decorated.decorator.range();
        let offer = offer(&decorated, &missing);
        if offer.as_ref().is_some_and(|offer| offer.unreported) {
            continue;
        }
        // The fix cannot take a key out of a splat, so adding the option would repeat it. The offer still runs for its
        // related locations and note.
        let passed = missing.iter().any(|option| splat_value(&decorated.splat, option).is_some());
        let offered = offer.map(|offer| Offer { options: if passed { Vec::new() } else { offer.options }, ..offer });
        let (fix, title, related, note) = match offered {
            // Rewriting the decorator would drop the comments inside it, so a commented decorator gets no fix.
            Some(Offer { options, edits, applicability, related, note, .. })
                if !options.is_empty() && !checker.has_comment_in(range) =>
            {
                let arguments = decorator_arguments(checker, decorated.keywords, &options);
                let edit = Edit::range_replacement(
                    format!("{}({})", checker.text(decorated.func), arguments.join(", ")),
                    range,
                );
                let fix = Fix::applicable_edits(edit, edits, applicability);
                (Some(fix), format!("Pass `{}` to `@dataclass`", settings(&options)), related, note)
            }
            Some(Offer { related, note, .. }) => (None, String::new(), related, note),
            None => (None, String::new(), Vec::new(), String::new()),
        };
        checker.report_related(
            rule,
            range,
            format!("Dataclass is missing {}; set `{}`{note}", missing.join(", "), settings(options)),
            fix,
            title,
            related,
        );
    }
}

struct Decorated<'a> {
    class: &'a ast::StmtClassDef,
    decorator: &'a Expr,
    func: &'a Expr,
    keywords: &'a [ast::Keyword],
    splat: Vec<(&'a str, &'a Expr)>,
    // A decorator below `@dataclass` runs first, and `slots=True` makes the dataclass return a new class, which the
    // decorator below never saw.
    wrapped: bool,
}

impl<'a> Decorated<'a> {
    // `None` for a class without a `@dataclass` decorator, or whose `**` splat has keys that cannot be read, as such a
    // splat may set any option.
    fn parse(checker: &'a Checker, class: &'a ast::StmtClassDef) -> Option<Self> {
        let is_dataclass = |func: &Expr| checker.semantic().resolves_to(func, &["dataclasses", "dataclass"]);
        let decorator = dataclass_decorator(class, &is_dataclass)?;
        let (func, keywords) = match decorator {
            Expr::Call(ast::ExprCall { func, arguments, .. }) => (func.as_ref(), &*arguments.keywords),
            other => (other, &[][..]),
        };
        Some(Self {
            class,
            decorator,
            func,
            keywords,
            splat: splatted_options(checker, keywords)?,
            wrapped: class.decorator_list.iter().any(|other| other.start() > decorator.end()),
        })
    }
}

// The options the `**` arguments among `keywords` pass with their values, or `None` when one of them cannot be read.
fn splatted_options<'s>(checker: &'s Checker, keywords: &'s [ast::Keyword]) -> Option<Vec<(&'s str, &'s Expr)>> {
    let mut passed = Vec::new();
    for keyword in keywords.iter().filter(|keyword| keyword.arg.is_none()) {
        passed.extend(splat_pairs(checker, &keyword.value)?);
    }
    // A key repeated across the splats is a runtime error, so no fix may rely on its value.
    let mut seen = FxHashSet::default();
    passed.iter().all(|(option, _)| seen.insert(*option)).then_some(passed)
}

// The pairs of a splat of a module-level name bound once, to a dict literal or `dict(...)` call.
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

impl<'a> Decorated<'a> {
    // The value the decorator passes for `name`, by keyword or by a splat. A splat cannot repeat a keyword's name.
    fn option(&self, name: &str) -> Option<&'a Expr> {
        self.keywords
            .iter()
            .find(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == name))
            .map(|keyword| &keyword.value)
            .or_else(|| splat_value(&self.splat, name))
    }
}

fn splat_value<'a>(splat: &[(&str, &'a Expr)], option: &str) -> Option<&'a Expr> {
    splat.iter().find(|(name, _)| *name == option).map(|(_, value)| *value)
}

// The options a fix sets, any edits it makes beyond the decorator, and how safe it is. With no options the report has
// no fix; it still points at `related` and adds `note` to its message. An `unreported` offer reports nothing: Ruff
// reports the class, or the option would break it on the target version.
struct Offer {
    options: Vec<&'static str>,
    edits: Vec<Edit>,
    applicability: Applicability,
    related: Vec<Related>,
    note: String,
    unreported: bool,
}

impl Offer {
    fn options(options: Vec<&'static str>, applicability: Applicability) -> Self {
        Self { options, edits: Vec::new(), applicability, related: Vec::new(), note: String::new(), unreported: false }
    }

    fn unreported() -> Self {
        Self { unreported: true, ..Self::options(Vec::new(), Applicability::Safe) }
    }
}

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

// `frozen` precedes `slots`; any other option has no rank and is appended.
fn option_rank(option: &str) -> Option<usize> {
    ["frozen", "slots"].iter().position(|name| *name == option)
}

fn settings(options: &[&str]) -> String {
    options.iter().map(|option| format!("{option}=True")).collect::<Vec<_>>().join(", ")
}
