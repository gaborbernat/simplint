use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::parenthesize::parenthesized_range;
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Expr, PythonVersion};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallKind, Use, Uses, dataclass_arguments, init_fields};
use crate::rules::ExprCollector;
use crate::rules::dataclasses::{Offer, require_options};
use crate::semantic::BindingKind;

pub fn dataclass_kw_only(checker: &Checker) {
    require_options(checker, Rule::DataclassKwOnly, &["kw_only"], |checker, decorated| {
        let edits = keyword_calls(checker, decorated.class)?;
        Some(Offer { options: decorated.missing.clone(), edits, applicability: Applicability::Unsafe })
    });
    if checker.settings.from_ruff.target_version >= PythonVersion::PY310 {
        report_positional_calls(checker);
    }
}

// The edits that pass every positional call of `class` in this module by keyword, or `None` when some use of the class
// could still pass fields by position once it is keyword-only: a call unpacking `*args`, the class handed
// around as a value, a subclass, or a positional call in another file that file's own report cannot rewrite. The other
// files' reports rewrite their calls in the same run.
fn keyword_calls(checker: &Checker, class: &ast::StmtClassDef) -> Option<Vec<Edit>> {
    let name = class.name.as_str();
    let semantic = checker.semantic();
    if !checker.body().iter().any(|stmt| stmt.as_class_def_stmt().is_some_and(|stmt| std::ptr::eq(stmt, class)))
        || semantic.is_rebound(name)
        || checker.project().is_base_class(name)
        || checker.project().dataclass_calls().has_opaque_use(name)
        || makes_own_instances(class)
    {
        return None;
    }
    let fields = positional_fields(checker, class)?;
    let references: Vec<_> = semantic.references_to(name).iter().map(|reference| reference.range).collect();
    let mut module = Uses::new(|expr: &Expr| {
        expr.as_name_expr().is_some_and(|found| references.contains(&found.range)).then_some(())
    });
    module.visit_body(checker.body());
    let mut methods =
        Uses::new(|expr: &Expr| expr.as_name_expr().is_some_and(|found| found.id.as_str() == "cls").then_some(()));
    methods.visit_body(&class.body);
    let mut edits = Vec::new();
    for ((), found) in module.found.into_iter().chain(methods.found) {
        let Use::Call(call) = found else {
            return None;
        };
        edits.extend(keyword_edits(checker, call, &fields)?);
    }
    Some(edits)
}

// `type(self)(...)` and `self.__class__(...)` call the class without naming it.
fn makes_own_instances(class: &ast::StmtClassDef) -> bool {
    let mut calls = ExprCollector::new(|expr: &Expr| {
        let func = &expr.as_call_expr()?.func;
        let is_type = func
            .as_call_expr()
            .is_some_and(|inner| inner.func.as_name_expr().is_some_and(|name| name.id.as_str() == "type"));
        let is_class = func.as_attribute_expr().is_some_and(|attribute| attribute.attr.as_str() == "__class__");
        (is_type || is_class).then_some(())
    });
    calls.visit_body(&class.body);
    !calls.found.is_empty()
}

// The fields the class takes by position, following its bases while they are dataclasses of this module.
fn positional_fields(checker: &Checker, class: &ast::StmtClassDef) -> Option<Vec<String>> {
    let is_dataclass = |expr: &Expr| checker.semantic().resolves_to(expr, &["dataclasses", "dataclass"]);
    let keywords = dataclass_arguments(class, &is_dataclass)?;
    let base_fields = |name: &str| {
        let base = checker
            .body()
            .iter()
            .filter_map(ast::Stmt::as_class_def_stmt)
            .find(|base| base.name.as_str() == name && base.end() <= class.start())?;
        positional_fields(checker, base)
    };
    init_fields(class, keywords, &base_fields)
}

// Each positional argument gets its field's name; a bare generator, the only argument, needs brackets once it has one.
fn keyword_edits(checker: &Checker, call: &ast::ExprCall, fields: &[String]) -> Option<Vec<Edit>> {
    match CallKind::of(call) {
        CallKind::Unpacked => return None,
        _ if call.arguments.args.len() > fields.len() => return None,
        _ => {}
    }
    let mut edits = Vec::new();
    for (argument, field) in call.arguments.args.iter().zip(fields) {
        let range =
            parenthesized_range(argument.into(), (&call.arguments).into(), checker.comment_ranges(), checker.source)
                .unwrap_or_else(|| argument.range());
        if argument.as_generator_expr().is_some_and(|generator| !generator.parenthesized) {
            edits.push(Edit::insertion(format!("{field}=("), range.start()));
            edits.push(Edit::insertion(")".to_owned(), range.end()));
        } else {
            edits.push(Edit::insertion(format!("{field}="), range.start()));
        }
    }
    Some(edits)
}

// In other files, the positional calls of dataclasses `dataclass-kw-only` asks to make keyword-only: the fix of the
// decorator relies on each file rewriting its own.
fn report_positional_calls(checker: &Checker) {
    let calls = checker.project().dataclass_calls();
    if !calls.calls_any() {
        return;
    }
    let semantic = checker.semantic();
    let mut uses = Uses::new(|expr: &Expr| {
        let name = UnqualifiedName::from_expr(expr)?;
        let written = name.segments().join(".");
        let fields = calls.fields(&written)?;
        let mut head = expr;
        while let Expr::Attribute(attribute) = head {
            head = &attribute.value;
        }
        let head = head.as_name_expr()?;
        let imported = semantic.references_to(head.id.as_str()).iter().any(|reference| reference.range == head.range)
            && semantic.bindings(head.id.as_str()).iter().all(|binding| binding.kind == BindingKind::Import);
        imported.then_some((written, fields))
    });
    uses.visit_body(checker.body());
    for ((written, fields), found) in uses.found {
        let Use::Call(call) = found else {
            continue;
        };
        if call.arguments.args.is_empty() {
            continue;
        }
        let fix = keyword_edits(checker, call, fields).map(|mut edits| {
            let first = edits.remove(0);
            Fix::safe_edits(first, edits)
        });
        checker.report_fix(
            Rule::DataclassKwOnly,
            call.range(),
            format!("Dataclass `{written}` is called with positional arguments; pass its fields by keyword"),
            fix,
            format!("Pass the fields of `{written}` by keyword"),
        );
    }
}
