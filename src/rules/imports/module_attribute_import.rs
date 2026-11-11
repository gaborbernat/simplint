use std::cell::OnceCell;

use itertools::Itertools;
use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::{Visibility, for_each_body, import_aliases};

// A module bound by an import of this run's own code is reached through an attribute, which the name itself reaches
// without the module.
pub fn module_attribute_import(checker: &Checker) {
    let semantic = checker.semantic();
    let uses = OnceCell::new();
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
        let uses = uses.get_or_init(|| Uses::of(checker.body()));
        let mut attributes = Vec::new();
        let mut value = false;
        for used in semantic.references_to(&module.bound).iter().filter_map(|reference| uses.0.get(&reference.range)) {
            match used {
                Use::Attribute(attribute) => attributes.push(attribute),
                Use::Value => value = true,
            }
        }
        if value {
            continue;
        }
        attributes.sort_unstable_by_key(|attribute| attribute.range.start());
        let path = module_path(statement, alias);
        // The fix replaces the import, so it is offered only when every use of the module is convertible: a single
        // import without a comment, no stores, no computed write the import would miss, each name importable from the
        // module, and no other module that imports or reads the bound name, which the replaced import would no longer
        // bind.
        let mut convertible = import_aliases(statement).len() == 1
            && !checker.has_comment_in(statement.range())
            && !module.computed_written
            && !checker.is_exported(&module.bound);
        let private_path = path.split('.').any(|part| Visibility::of(part) == Visibility::Private);
        let mut reports = Vec::new();
        for attribute in attributes {
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
            // A builtin's name would shadow the builtin the module may still call once the name is imported.
            convertible &= !private_path
                && Visibility::of(attribute.name) != Visibility::Private
                && module.defined.contains(attribute.name)
                && !is_shadowed(checker, attribute)
                && !rebound
                && !is_python_builtin(attribute.name, checker.target_version().minor, false);
            reports.push(attribute);
        }
        let shared = reports
            .iter()
            .map(|attribute| {
                let message =
                    format!("Import `{}` from `{path}` instead of reaching it through the module", attribute.name);
                (attribute.range, message, format!("Import the names from `{path}`"))
            })
            .collect();
        checker.report_shared_fix(Rule::ModuleAttributeImport, shared, || {
            convertible.then(|| {
                let names: Vec<&str> = reports.iter().map(|attribute| attribute.name).unique().collect();
                let import = Edit::range_replacement(
                    format!("{}from {path} import {}", lazy_prefix(statement), names.join(", ")),
                    statement.range(),
                );
                let accesses =
                    reports.iter().map(|attribute| Edit::range_replacement(attribute.name.to_owned(), attribute.range));
                Fix::unsafe_edits(import, accesses)
            })
        });
    }
}

// Whether a local binding of the attribute's name shares the top-level statement of the use, so the bare name may
// resolve to it: a parameter, a local, a comprehension target, a match capture or a type parameter.
fn is_shadowed(checker: &Checker, attribute: &Attribute) -> bool {
    let body = checker.body();
    let index = body.partition_point(|stmt| stmt.end() <= attribute.range.start());
    body.get(index).filter(|stmt| stmt.range().contains_range(attribute.range)).is_some_and(|stmt| {
        checker.semantic().local_bindings(attribute.name).iter().any(|binding| stmt.range().contains_range(*binding))
    })
}

// The import statement and alias that create the module-level binding at `binding`, so a same-named import in a function
// stays out of it.
fn binding_import(body: &[Stmt], binding: TextRange) -> Option<(&Stmt, &ast::Alias)> {
    let mut found = None;
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            if let Some(alias) = import_aliases(stmt).iter().find(|alias| alias.range.contains_range(binding)) {
                found = Some((stmt, alias));
            }
        }
    });
    found
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

// How each name of a file is used, by the name's range: as the object of an attribute access, or as a value. One walk
// serves every imported module, whose references the semantic model resolves, so a parameter or local of the same name
// is not a use of it.
struct Uses<'a>(FxHashMap<TextRange, Use<'a>>);

impl<'a> Uses<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut uses = Self(FxHashMap::default());
        uses.visit_body(body);
        uses
    }
}

enum Use<'a> {
    Attribute(Attribute<'a>),
    Value,
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
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx, range, .. }) if let Expr::Name(name) = &**value => {
                let attribute =
                    Attribute { range: *range, name: attr.as_str(), load: matches!(ctx, ExprContext::Load) };
                self.0.insert(name.range, Use::Attribute(attribute));
                return;
            }
            Expr::Name(name) => {
                self.0.insert(name.range, Use::Value);
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
