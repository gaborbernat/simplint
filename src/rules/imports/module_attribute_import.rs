use itertools::Itertools;
use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{Visibility, for_each_body};

// A module bound by an import of this run's own code is reached through an attribute, which the name itself reaches
// without the module.
pub fn module_attribute_import(checker: &Checker) {
    let semantic = checker.semantic();
    for module in checker.project().modules() {
        if checker.settings.project.knobs.module_attributes.contains(&module.module) {
            continue;
        }
        // Bound twice: its attributes cannot move into the importer.
        let [binding] = semantic.bindings(&module.bound) else {
            continue;
        };
        let (statement, alias) =
            binding_import(checker.body(), binding.range).expect("a module binding is created by an import statement");
        // A package read through its own alias, as `pd.NaT` for `import pandas as pd`, is how the package is meant to
        // be used, so only the reads of its submodules are reported.
        if module.top_level_package && alias.asname.as_ref().is_some_and(|asname| asname.as_str() != module.module) {
            continue;
        }
        let loads = semantic.references_to(&module.bound).iter().map(|reference| reference.range).collect();
        let mut uses = Uses { loads, attributes: Vec::new(), value: false };
        uses.visit_body(checker.body());
        if uses.value {
            continue;
        }
        let path = module_path(statement, alias);
        // The fix replaces the import, so it is offered only when every use of the module is convertible: a single
        // import, no stores, no computed write the import would miss, and each name importable from the module.
        let mut convertible = aliases(statement).len() == 1 && !module.computed_written;
        let private_path = path.split('.').any(|part| Visibility::of(part) == Visibility::Private);
        let mut reports = Vec::new();
        for attribute in &uses.attributes {
            let reportable = attribute.load && !is_dunder(attribute.name) && !semantic.is_bound(attribute.name);
            // Only a name another file patches or stores stays on the module; its other attributes still move.
            if !reportable || module.patched.contains(attribute.name) {
                convertible = false;
                continue;
            }
            // A function that rebinds the name through `global`, or a store to it anywhere in the run, changes the
            // attribute after the import would have bound it.
            let rebound =
                module.rebound.contains(attribute.name) || checker.project().is_attribute_assigned(attribute.name);
            convertible &= !private_path
                && Visibility::of(attribute.name) != Visibility::Private
                && module.defined.contains(attribute.name)
                && !is_shadowed(checker, attribute)
                && !rebound;
            reports.push(attribute);
        }
        let fix = convertible.then(|| {
            let names: Vec<&str> = reports.iter().map(|attribute| attribute.name).unique().collect();
            let import = Edit::range_replacement(
                format!("{}from {path} import {}", lazy_prefix(statement), names.join(", ")),
                statement.range(),
            );
            let accesses =
                reports.iter().map(|attribute| Edit::range_replacement(attribute.name.to_owned(), attribute.range));
            Fix::unsafe_edits(import, accesses)
        });
        for attribute in reports {
            let message =
                format!("Import `{}` from `{path}` instead of reaching it through the module", attribute.name);
            match &fix {
                Some(fix) => checker.report_fix(
                    Rule::ModuleAttributeImport,
                    attribute.range,
                    message,
                    Some(fix.clone()),
                    format!("Import the names from `{path}`"),
                ),
                None => checker.report(Rule::ModuleAttributeImport, attribute.range, message),
            }
        }
    }
}

// Whether a local binding of the attribute's name shares the top-level statement of the use, so the bare name may
// resolve to it: a parameter, a local, a comprehension target, a match capture or a type parameter.
fn is_shadowed(checker: &Checker, attribute: &Attribute) -> bool {
    checker.body().iter().find(|stmt| stmt.range().contains_range(attribute.range)).is_some_and(|stmt| {
        checker.semantic().local_bindings(attribute.name).iter().any(|binding| stmt.range().contains_range(*binding))
    })
}

// The import statement and alias that create the module-level binding at `binding`, so a same-named import in a function
// stays out of it.
fn binding_import(body: &[Stmt], binding: TextRange) -> Option<(&Stmt, &ast::Alias)> {
    let mut found = None;
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            if let Some(alias) = aliases(stmt).iter().find(|alias| alias.range.contains_range(binding)) {
                found = Some((stmt, alias));
            }
        }
    });
    found
}

// The aliases an import statement binds, and none for any other statement.
fn aliases(stmt: &Stmt) -> &[ast::Alias] {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => names,
        _ => &[],
    }
}

// The `lazy` keyword of a lazy import, which the rewritten import keeps so the module still loads on first use.
fn lazy_prefix(stmt: &Stmt) -> &'static str {
    match stmt {
        Stmt::Import(ast::StmtImport { is_lazy: true, .. })
        | Stmt::ImportFrom(ast::StmtImportFrom { is_lazy: true, .. }) => "lazy ",
        _ => "",
    }
}

// The module as the import spells it, relative with its dots, such as `.findings` for `from . import findings`.
fn module_path(stmt: &Stmt, alias: &ast::Alias) -> String {
    match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, .. }) => {
            let parts: Vec<&str> = module.iter().map(ast::Identifier::as_str).chain([alias.name.as_str()]).collect();
            format!("{}{}", ".".repeat(*level as usize), parts.join("."))
        }
        _ => alias.name.to_string(),
    }
}

// Where a module binding is used in a file: its attribute accesses, and whether it is used as a value. Only the loads
// the semantic model resolves to the module count, so a parameter or local of the same name is not a use of it.
struct Uses<'a> {
    loads: FxHashSet<TextRange>,
    attributes: Vec<Attribute<'a>>,
    value: bool,
}

struct Attribute<'a> {
    range: TextRange,
    name: &'a str,
    // Whether the access reads the attribute, rather than storing or deleting it.
    load: bool,
}

impl<'a> Visitor<'a> for Uses<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx, range, .. })
                if value.as_name_expr().is_some_and(|name| self.loads.contains(&name.range)) =>
            {
                self.attributes.push(Attribute {
                    range: *range,
                    name: attr.as_str(),
                    load: matches!(ctx, ExprContext::Load),
                });
                return;
            }
            Expr::Name(name) if self.loads.contains(&name.range) => self.value = true,
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
