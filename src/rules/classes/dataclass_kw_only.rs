use std::cell::OnceCell;

use ruff_diagnostics::{Applicability, Edit};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_python_parser::parse_string_annotation;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::lint::checker::{Checker, Related};
use crate::lint::noqa;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallKind, CallSite, Use, Uses, init_fields};
use crate::rules::classes::{Offer, require_options, splatted_options};
use crate::rules::edits::keep_debug_text;
use crate::syntax::ExprCollector;
use crate::syntax::{dataclass_decorator, is_classmethod, is_name, receiver};

// `slots=`, `kw_only=` and the keyword calls the rule writes arrived in Python 3.10.
pub(crate) const SINCE: PythonVersion = PythonVersion::PY310;

pub fn dataclass_kw_only(checker: &Checker) {
    let module = OnceCell::new();
    require_options(checker, Rule::DataclassKwOnly, &["kw_only"], |decorated, missing| {
        let module = module.get_or_init(|| ModuleClasses::of(checker));
        let class = decorated.class;
        let name = class.name.as_str();
        // The report names every positional call of the class, here and in other files; the fix needs all of them in
        // this file, since a rewrite does not reach another file.
        let mut related: Vec<Related> = positional_calls(checker.source, module, class)
            .into_iter()
            .map(|range| Related { path: checker.path.to_path_buf(), range, message: CALL_MESSAGE.to_owned() })
            .collect();
        let calls = checker.project().calls().positional_calls(name);
        // A call that suppresses this rule leaves the report, but still withholds the fix, which would pass that call
        // by keyword anyway, against the comment's wish.
        let others: Vec<&CallSite> = calls
            .iter()
            .filter(|site| !site.comments.iter().any(|comment| noqa::suppresses(comment, Rule::DataclassKwOnly.code())))
            .collect();
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
        let edits = if calls.is_empty() { keyword_calls(checker, module, class) } else { None };
        let options = if edits.is_some() { missing.to_vec() } else { Vec::new() };
        Some(Offer {
            edits: edits.unwrap_or_default(),
            related,
            note,
            ..Offer::options(options, Applicability::Unsafe)
        })
    });
}

// The message of each positional call a dataclass report points at.
const CALL_MESSAGE: &str = "positional arguments pass the fields by position";

// The calls in this module that pass the class fields by position: the class's own calls, and `cls(...)` in its
// classmethods.
fn positional_calls(source: &str, module: &ModuleClasses, class: &ast::StmtClassDef) -> Vec<TextRange> {
    let module_uses = module.uses.get(class.name.as_str()).map_or(&[][..], Vec::as_slice);
    module_uses
        .iter()
        .chain(&receiver_calls(source, class))
        .filter_map(|found| match found {
            Use::Call(call) if !call.arguments.args.is_empty() => Some(call.range()),
            _ => None,
        })
        .collect()
}

// The calls a classmethod makes of its first parameter, which is the class. A method's first parameter is the instance
// or something else, so a `cls` there is not the class.
fn receiver_calls<'a>(source: &'a str, class: &'a ast::StmtClassDef) -> Vec<Use<'a>> {
    let mut found = Vec::new();
    for method in class.body.iter().filter_map(Stmt::as_function_def_stmt).filter(|method| is_classmethod(method)) {
        let Some(cls) = receiver(method) else {
            continue;
        };
        let mut calls = Uses::new(|expr: &Expr| is_name(expr, cls).then_some(())).with_source(source);
        calls.visit_body(&method.body);
        found.extend(calls.found.into_iter().map(|((), found)| found));
    }
    found
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
    let module_uses = module.uses.get(name).map_or(&[][..], Vec::as_slice);
    let mut edits = Vec::new();
    for found in module_uses.iter().chain(&receiver_calls(checker.source, class)) {
        let Use::Call(call) = found else {
            return None;
        };
        edits.extend(keyword_edits(checker, call, &fields)?);
    }
    keep_debug_text(checker, edits)
}

// The module-level class names a string annotation spells, with where each is. Ruff resolves no reference to them, so
// they are matched by name, as the string is read as an expression of the module.
struct StringNames<'c, 'a> {
    source: &'c str,
    classes: &'c FxHashMap<&'a str, Vec<&'a ast::StmtClassDef>>,
    found: Vec<(TextRange, &'a str)>,
}

impl<'a> Visitor<'a> for StringNames<'_, 'a> {
    fn visit_annotation(&mut self, annotation: &'a Expr) {
        let mut strings = ExprCollector::new(|expr: &'a Expr| expr.as_string_literal_expr());
        strings.visit_expr(annotation);
        for literal in strings.found {
            let Some(string) = literal.as_single_part_string() else {
                continue;
            };
            let Ok(parsed) = parse_string_annotation(self.source, string) else {
                continue;
            };
            let classes = self.classes;
            let mut names = ExprCollector::new(|expr: &Expr| {
                let name = expr.as_name_expr()?;
                let (class, _) = classes.get_key_value(name.id.as_str())?;
                Some((name.range, *class))
            });
            names.visit_expr(parsed.expr());
            self.found.extend(names.found);
        }
    }
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
        let mut references: FxHashMap<TextRange, &str> = classes
            .keys()
            .flat_map(|name| semantic.references_to(name).iter().map(|reference| (reference.range, *name)))
            .collect();
        let mut strings = StringNames { source: checker.source, classes: &classes, found: Vec::new() };
        strings.visit_body(checker.body());
        references.extend(strings.found);
        let mut walk =
            Uses::new(|expr: &Expr| references.get(&expr.as_name_expr()?.range).copied()).with_source(checker.source);
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

// Each positional argument gets its field's name; a bare generator or walrus needs brackets as a keyword's value.
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
        let needs_parentheses = match argument {
            Expr::Generator(generator) => !generator.parenthesized,
            Expr::Named(_) => range == argument.range(),
            _ => false,
        };
        if needs_parentheses {
            edits.push(Edit::insertion(format!("{field}=("), range.start()));
            edits.push(Edit::insertion(")".to_owned(), range.end()));
        } else {
            edits.push(Edit::insertion(format!("{field}="), range.start()));
        }
    }
    Some(edits)
}
