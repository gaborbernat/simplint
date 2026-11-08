use std::cell::OnceCell;

use ruff_diagnostics::{Applicability, Edit};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::lint::checker::{Checker, Related};
use crate::lint::noqa;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallKind, Use, Uses, init_fields};
use crate::rules::classes::{Offer, require_options, splatted_options};
use crate::rules::edits::keep_debug_text;
use crate::syntax::ExprCollector;
use crate::syntax::{dataclass_decorator, is_name};

// `slots=`, `kw_only=` and the keyword calls the rule writes arrived in Python 3.10.
pub(crate) const SINCE: PythonVersion = PythonVersion::PY310;

// Whether a comment on a positional call withholds the class's fix, since the rewrite would pass that call by keyword
// anyway, against the comment's wish.
pub(crate) fn suppresses_rewrite(comment: &str) -> bool {
    noqa::suppresses(comment, Rule::DataclassKwOnly.code())
}

pub fn dataclass_kw_only(checker: &Checker) {
    let module = OnceCell::new();
    require_options(checker, Rule::DataclassKwOnly, &["kw_only"], |decorated, missing| {
        let module = module.get_or_init(|| ModuleClasses::of(checker));
        let class = decorated.class;
        let name = class.name.as_str();
        // The report names every positional call of the class, here and in other files; the fix needs all of them in
        // this file, since a rewrite does not reach another file.
        let mut related: Vec<Related> = positional_calls(module, class)
            .into_iter()
            .map(|range| Related { path: checker.path.to_path_buf(), range, message: CALL_MESSAGE.to_owned() })
            .collect();
        let others = checker.project().calls().positional_calls(name);
        related.extend(others.iter().map(|site| Related {
            path: site.caller.clone(),
            range: TextRange::new(TextSize::new(site.start), TextSize::new(site.end)),
            message: CALL_MESSAGE.to_owned(),
        }));
        let note = match others.len() {
            0 => String::new(),
            1 => "; 1 positional call in another file needs keyword arguments by hand".to_owned(),
            count => format!("; {count} positional calls in other files need keyword arguments by hand"),
        };
        let edits = if others.is_empty() { keyword_calls(checker, module, class) } else { None };
        let options = if edits.is_some() { missing.to_vec() } else { Vec::new() };
        Some(Offer {
            options,
            edits: edits.unwrap_or_default(),
            applicability: Applicability::Unsafe,
            related,
            note,
            left_to_ruff: false,
        })
    });
}

// The message of each positional call a dataclass report points at.
const CALL_MESSAGE: &str = "positional arguments pass the fields by position";

// The calls in this module that pass the class fields by position: the class's own calls, and `cls(...)` in its methods.
fn positional_calls(module: &ModuleClasses, class: &ast::StmtClassDef) -> Vec<TextRange> {
    let mut methods = Uses::new(|expr: &Expr| is_name(expr, "cls").then_some(()));
    methods.visit_body(&class.body);
    let module_uses = module.uses.get(class.name.as_str()).map_or(&[][..], Vec::as_slice);
    module_uses
        .iter()
        .chain(methods.found.iter().map(|((), found)| found))
        .filter_map(|found| match found {
            Use::Call(call) if !call.arguments.args.is_empty() => Some(call.range()),
            _ => None,
        })
        .collect()
}

// The edits that pass every positional call of `class` in this module by keyword, or `None` when some use of the class
// could still pass fields by position once it is keyword-only: a call unpacking `*args`, the class handed around as a
// value, or a subclass.
fn keyword_calls(checker: &Checker, module: &ModuleClasses, class: &ast::StmtClassDef) -> Option<Vec<Edit>> {
    let name = class.name.as_str();
    if !module.is_module_level(class)
        || checker.semantic().is_rebound(name)
        || checker.project().is_base_class(name)
        || checker.project().calls().has_opaque_use(name)
        || makes_own_instances(class)
    {
        return None;
    }
    let fields = positional_fields(checker, module, class)?;
    let mut methods = Uses::new(|expr: &Expr| is_name(expr, "cls").then_some(()));
    methods.visit_body(&class.body);
    let module_uses = module.uses.get(name).map_or(&[][..], Vec::as_slice);
    let mut edits = Vec::new();
    for found in module_uses.iter().chain(methods.found.iter().map(|((), found)| found)) {
        let Use::Call(call) = found else {
            return None;
        };
        edits.extend(keyword_edits(checker, call, &fields)?);
    }
    keep_debug_text(checker, edits)
}

// The module-level classes by name in source order, and every use of them in the module, found in one walk.
struct ModuleClasses<'a> {
    classes: FxHashMap<&'a str, Vec<&'a ast::StmtClassDef>>,
    uses: FxHashMap<&'a str, Vec<Use<'a>>>,
}

impl<'a> ModuleClasses<'a> {
    fn of(checker: &Checker<'a>) -> Self {
        let mut classes: FxHashMap<&str, Vec<&ast::StmtClassDef>> = FxHashMap::default();
        for class in checker.body().iter().filter_map(ast::Stmt::as_class_def_stmt) {
            classes.entry(class.name.as_str()).or_default().push(class);
        }
        let semantic = checker.semantic();
        let references: FxHashMap<TextRange, &str> = classes
            .keys()
            .flat_map(|name| semantic.references_to(name).iter().map(|reference| (reference.range, *name)))
            .collect();
        let mut walk = Uses::new(|expr: &Expr| references.get(&expr.as_name_expr()?.range).copied());
        walk.visit_body(checker.body());
        let mut uses: FxHashMap<&str, Vec<Use>> = FxHashMap::default();
        for (name, found) in walk.found {
            uses.entry(name).or_default().push(found);
        }
        Self { classes, uses }
    }

    fn is_module_level(&self, class: &ast::StmtClassDef) -> bool {
        self.classes
            .get(class.name.as_str())
            .is_some_and(|classes| classes.iter().any(|found| std::ptr::eq(*found, class)))
    }
}

// `type(self)(...)` and `self.__class__(...)` call the class without naming it.
fn makes_own_instances(class: &ast::StmtClassDef) -> bool {
    let mut calls = ExprCollector::new(|expr: &Expr| {
        let func = &expr.as_call_expr()?.func;
        let is_type = func.as_call_expr().is_some_and(|inner| is_name(&inner.func, "type"));
        let is_class = func.as_attribute_expr().is_some_and(|attribute| attribute.attr.as_str() == "__class__");
        (is_type || is_class).then_some(())
    });
    calls.visit_body(&class.body);
    !calls.found.is_empty()
}

// The fields the class takes by position, following its bases while they are dataclasses of this module.
fn positional_fields(checker: &Checker, module: &ModuleClasses, class: &ast::StmtClassDef) -> Option<Vec<String>> {
    let is_dataclass = |expr: &Expr| checker.semantic().resolves_to(expr, &["dataclasses", "dataclass"]);
    let decorator = dataclass_decorator(class, &is_dataclass)?;
    let base_fields = |name: &str| {
        let base = module.classes.get(name)?.iter().find(|base| base.end() <= class.start())?;
        positional_fields(checker, module, base)
    };
    init_fields(class, decorator, &base_fields)
}

// Each positional argument gets its field's name; a bare generator, the only argument, needs brackets once it has one.
fn keyword_edits(checker: &Checker, call: &ast::ExprCall, fields: &[String]) -> Option<Vec<Edit>> {
    match CallKind::of(call) {
        CallKind::Unpacked => return None,
        _ if call.arguments.args.len() > fields.len() => return None,
        _ => {}
    }
    // A `**` splat that passes one of the fields the call now names would repeat it, and one whose keys are unknown
    // may pass any.
    let named = &fields[..call.arguments.args.len()];
    if !named.is_empty()
        && splatted_options(checker, &call.arguments.keywords)
            .is_none_or(|passed| passed.iter().any(|(name, _)| named.iter().any(|field| field == *name)))
    {
        return None;
    }
    let mut edits = Vec::new();
    for (argument, field) in call.arguments.args.iter().zip(fields) {
        let range = checker.parenthesized_range(argument, (&call.arguments).into());
        if argument.as_generator_expr().is_some_and(|generator| !generator.parenthesized) {
            edits.push(Edit::insertion(format!("{field}=("), range.start()));
            edits.push(Edit::insertion(")".to_owned(), range.end()));
        } else {
            edits.push(Edit::insertion(format!("{field}="), range.start()));
        }
    }
    Some(edits)
}
