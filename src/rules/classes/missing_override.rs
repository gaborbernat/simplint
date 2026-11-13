use std::cell::OnceCell;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{ensure_import, with_import};
use crate::syntax::module_statements;

pub fn missing_override(checker: &Checker) {
    let project = checker.project();
    for class in module_statements(checker.body()).filter_map(Stmt::as_class_def_stmt) {
        let methods: Vec<&ast::StmtFunctionDef> = class.body.iter().filter_map(Stmt::as_function_def_stmt).collect();
        let marked = OnceCell::new();
        let mut seen: FxHashSet<&str> = FxHashSet::default();
        for &method in &methods {
            let name = method.name.as_str();
            // The checks that hold for every definition of the name come first, as they skip most methods without
            // resolving a decorator. Every class inherits the special methods from `object`, so overriding one is how a
            // class takes part in a protocol. Overloads leave the decorator to the implementation, and a property's
            // setter to its getter, the first definition of the name.
            if is_dunder(name)
                || !project.is_override(class.name.as_str(), name)
                || marked.get_or_init(|| marked_names(checker, &methods)).contains(name)
                || is_overload(checker, method)
                || !seen.insert(name)
            {
                continue;
            }
            checker.report_fix(
                Rule::MissingOverride,
                method.name.range(),
                format!("`{name}` overrides a method of a base class of `{}` without `@override`", class.name),
                fix(checker, method),
                "Mark it with `@override`",
            );
        }
    }
}

// The decorator goes next to the `def`, below the others, so `@property` or `@classmethod` wraps the marked function as
// type checkers expect.
fn fix(checker: &Checker, method: &ast::StmtFunctionDef) -> Option<Fix> {
    let import = ensure_import(checker, "typing", Some("override"), method.range())?;
    let decorator = format!("{}@override{}", checker.indentation(method), checker.line_ending());
    let edit = Edit::insertion(decorator, checker.source.line_start(method.name.start()));
    Some(with_import(edit, import, Applicability::Safe))
}

fn marked_names<'a>(checker: &Checker, methods: &[&'a ast::StmtFunctionDef]) -> FxHashSet<&'a str> {
    methods
        .iter()
        .filter(|method| {
            method
                .decorator_list
                .iter()
                .any(|decorator| checker.semantic().match_typing_expr(&decorator.expression, "override"))
        })
        .map(|method| method.name.as_str())
        .collect()
}

fn is_overload(checker: &Checker, method: &ast::StmtFunctionDef) -> bool {
    method
        .decorator_list
        .iter()
        .any(|decorator| checker.semantic().match_typing_expr(&decorator.expression, "overload"))
}
