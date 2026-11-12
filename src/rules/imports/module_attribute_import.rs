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
use crate::syntax::{Visibility, for_each_body, import_aliases, is_lazy, is_stdlib_root};

pub fn module_attribute_import(checker: &Checker) {
    let semantic = checker.semantic();
    let knobs = &checker.settings.project.knobs;
    let scope = Scope::of(&knobs.module_attribute_scope);
    let uses = OnceCell::new();
    for module in checker.project().modules() {
        if !(scope.project || scope.names(&module.module)) || knobs.module_attributes.contains(&module.module) {
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
        // The fix replaces the import, so it is offered only when every use of the module is convertible: a single
        // import without a comment, no stores, no computed write the import would miss, each name importable from the
        // module, and no other module that imports or reads the bound name, which the replaced import would no longer
        // bind.
        let mut convertible = import_aliases(statement).len() == 1
            && !checker.has_comment_in(statement.range())
            && !module.computed_written
            && !checker.is_exported(&module.bound);
        let path = module_path(statement, alias);
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
    if scope.reaches_outside() {
        outside_modules(checker, &scope, &uses);
    }
}

// A module outside the run reports its attributes without a fix: the run does not show which names it defines, so an
// import of one might fail or bind a submodule.
fn outside_modules<'a>(checker: &'a Checker, scope: &Scope, uses: &OnceCell<Uses<'a>>) {
    let semantic = checker.semantic();
    let knobs = &checker.settings.project.knobs;
    let from_ruff = &checker.settings.from_ruff;
    let minor = checker.target_version().minor;
    let run = checker.project().modules();
    for stmt in checker.body() {
        let Stmt::Import(ast::StmtImport { names, .. }) = stmt else {
            continue;
        };
        for alias in names {
            let module = alias.name.as_str();
            // `import os.path` binds `os`, whose `path` attribute is the imported submodule, not a name to import.
            let bound = match &alias.asname {
                Some(asname) => asname.as_str(),
                None if module.contains('.') => continue,
                None => module,
            };
            let covered = if is_stdlib_root(minor, module) { scope.stdlib } else { scope.third_party };
            if !(covered || scope.names(module))
                || run.iter().any(|used| used.bound == bound || used.module == module)
                || knobs.module_attributes.iter().any(|listed| listed == module)
                || from_ruff.import_aliases.get(module).is_some_and(|conventional| conventional == bound)
                || from_ruff.banned_from.contains(module)
            {
                continue;
            }
            let [_] = semantic.bindings(bound) else {
                continue;
            };
            let uses = uses.get_or_init(|| Uses::of(checker.body()));
            let mut attributes = Vec::new();
            for used in semantic.references_to(bound).iter().filter_map(|reference| uses.0.get(&reference.range)) {
                match used {
                    Use::Attribute(attribute) => attributes.push(attribute),
                    Use::Value => {
                        attributes.clear();
                        break;
                    }
                }
            }
            for attribute in attributes {
                if attribute.load
                    && !is_dunder(attribute.name)
                    && !semantic.is_bound(attribute.name)
                    && !checker.project().is_path_referenced(module, attribute.name)
                {
                    let message = format!(
                        "Import `{}` from `{module}` instead of reaching it through the module",
                        attribute.name
                    );
                    checker.report(Rule::ModuleAttributeImport, attribute.range, message);
                }
            }
        }
    }
}

// The modules `module-attribute-scope` lets the rule check.
struct Scope<'a> {
    project: bool,
    stdlib: bool,
    third_party: bool,
    modules: Vec<&'a str>,
}

impl<'a> Scope<'a> {
    fn of(entries: &'a [String]) -> Self {
        let mut scope = Self { project: false, stdlib: false, third_party: false, modules: Vec::new() };
        for entry in entries {
            match entry.as_str() {
                "project" => scope.project = true,
                "stdlib" => scope.stdlib = true,
                "third-party" => scope.third_party = true,
                module => scope.modules.push(module),
            }
        }
        scope
    }

    fn reaches_outside(&self) -> bool {
        self.stdlib || self.third_party || !self.modules.is_empty()
    }

    // A listed module covers its submodules too.
    fn names(&self, module: &str) -> bool {
        self.modules
            .iter()
            .any(|listed| module.strip_prefix(listed).is_some_and(|rest| rest.is_empty() || rest.starts_with('.')))
    }
}

// The import that creates the module-level binding at `binding`, so a same-named import in a function stays out of it.
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

// One walk serves every imported module: the semantic model resolves each reference, so a parameter or local of the
// same name is not a use of it.
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

fn module_path(stmt: &Stmt, alias: &ast::Alias) -> String {
    match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { module, level, .. }) => {
            let parts: Vec<&str> = module.iter().map(ast::Identifier::as_str).chain([alias.name.as_str()]).collect();
            format!("{}{}", ".".repeat(*level as usize), parts.join("."))
        }
        _ => alias.name.to_string(),
    }
}

// A local binding in the same top-level statement, such as a parameter, can make the bare name resolve to it.
fn is_shadowed(checker: &Checker, attribute: &Attribute) -> bool {
    let body = checker.body();
    let index = body.partition_point(|stmt| stmt.end() <= attribute.range.start());
    body.get(index).filter(|stmt| stmt.range().contains_range(attribute.range)).is_some_and(|stmt| {
        checker.semantic().local_bindings(attribute.name).iter().any(|binding| stmt.range().contains_range(*binding))
    })
}

// The `lazy` keyword of a lazy import, which the rewritten import keeps so the module still loads on first use.
fn lazy_prefix(stmt: &Stmt) -> &'static str {
    if is_lazy(stmt) { "lazy " } else { "" }
}
