use rustc_hash::{FxHashMap, FxHashSet};

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextRange};

use super::lazy_modules::{
    Declaration, Imported, Item, Layout, LazyList, Module, declaration, enclosing_packages, imported_modules,
    list_position, module_imports, sort_key,
};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::semantic::{Evaluation, Reference, future_import};
use crate::syntax::{is_lazy, is_main_test, is_type_checking_name, signature_annotations};

struct Missing {
    range: TextRange,
    module: Module,
    key: Module,
}

pub fn missing_lazy_module(checker: &Checker) {
    // For Python 3.15 targets, prefer the explicit `lazy` syntax.
    if checker.target_version() >= PythonVersion::PY315 {
        return;
    }
    let declaration = declaration(checker);
    let list = match &declaration {
        Declaration::Listed(list) => Some(list),
        Declaration::Absent if checker.settings.project.knobs.require_lazy_modules => None,
        Declaration::Absent | Declaration::Unknown => return,
    };
    let module_path = checker.module_path();
    let listed: FxHashSet<Module> = list
        .iter()
        .flat_map(|list| &list.entries)
        .filter(|entry| !entry.written_relative)
        .map(|entry| entry.module.clone().resolve(module_path))
        .collect();
    let enclosing = enclosing_packages(module_path);
    let load_time = load_time_names(checker);
    let semantic = checker.semantic();
    let imports = module_imports(checker.body());
    let deferred: Vec<(bool, bool, Imported, Module)> = imports
        .iter()
        .filter(|import| import.governed && !import.dead && !is_lazy(import.stmt))
        .flat_map(|import| {
            // An import above the list has loaded before the list exists, so listing its module defers nothing.
            let deferrable = list.is_none_or(|list| import.stmt.start() > list.stmt.start());
            imported_modules(import.stmt).into_iter().map(move |imported| {
                let key = imported.module.clone().resolve(module_path);
                (import.top, deferrable, imported, key)
            })
        })
        .collect();
    // `import pkg` of a package holding the file binds what has already loaded, so it runs nothing a lazy import would
    // skip, while `from pkg import name` can still load a submodule on first use.
    let holds_file = |imported: &Imported, key: &Module| !imported.from_import && enclosing.contains(key);
    // Each entry affects all matching imports. An unread, unexported import may be needed for its side effects.
    let mut waits: FxHashMap<&Module, bool> = FxHashMap::default();
    for (_, deferrable, imported, key) in &deferred {
        let binding = imported.binding;
        let waits_for_use = *deferrable
            && !load_time.contains(binding)
            && !semantic.is_rebound(binding)
            && (holds_file(imported, key)
                || !semantic.references_to(binding).is_empty()
                || checker.is_exported(binding));
        *waits.entry(key).or_insert(true) &= waits_for_use;
    }
    let mut missing: Vec<Missing> = Vec::new();
    for (_, _, imported, key) in deferred.iter().filter(|(top, _, imported, key)| *top && !holds_file(imported, key)) {
        if waits[key]
            && !listed.contains(key)
            && !key.is_loaded_at_startup()
            && missing.iter().all(|seen| seen.key != *key)
        {
            missing.push(Missing { range: imported.range, module: imported.module.clone(), key: key.clone() });
        }
    }
    if missing.is_empty() {
        return;
    }
    let reports = missing
        .iter()
        .map(|missing| {
            (
                missing.range,
                format!(
                    "`{}` is first used after the module loads; list it in `__lazy_modules__` to load it then",
                    missing.key
                ),
                format!("Add `{}` to `__lazy_modules__`", missing.key),
            )
        })
        .collect();
    checker.report_shared_fix(Rule::MissingLazyModule, reports, || {
        let quote = list.map_or_else(|| checker.stylist.quote().as_char(), |list| list.quote(checker));
        let mut added: Vec<Item> =
            missing.iter().map(|missing| Item { text: missing.module.entry(quote), comment: None }).collect();
        added.sort_by(|left, right| sort_key(&left.text).cmp(&sort_key(&right.text)));
        let edit = match list {
            Some(list) => extended(checker, list, added)?,
            None => new_list(checker, &added)?,
        };
        Some(Fix::unsafe_edit(edit))
    });
}

fn extended(checker: &Checker, list: &LazyList, added: Vec<Item>) -> Option<Edit> {
    let mut items = list.items(checker)?;
    let sorted = items.is_sorted_by(|left, right| sort_key(&left.text) <= sort_key(&right.text));
    for item in added {
        let at = if sorted {
            items.iter().position(|existing| sort_key(&existing.text) > sort_key(&item.text)).unwrap_or(items.len())
        } else {
            items.len()
        };
        items.insert(at, item);
    }
    list.replacement(checker, &items)
}

fn new_list(checker: &Checker, items: &[Item]) -> Option<Edit> {
    let first = checker
        .body()
        .iter()
        .find(|stmt| (stmt.is_import_stmt() || stmt.is_import_from_stmt()) && future_import(stmt).is_none())
        .expect("every reported module comes from an import below the `from __future__` ones");
    let target = "__lazy_modules__ = ";
    let layout = Layout { brackets: ("[", "]"), single_tuple: false, base: "", used: target.len(), spans_lines: false };
    let eol = checker.line_ending();
    Some(Edit::insertion(format!("{target}{}{eol}{eol}", layout.text(checker, items)), list_position(checker, first)?))
}

// A call made during module loading can resolve an import inside its function body. Follow local calls transitively.
fn load_time_names<'a>(checker: &Checker<'a>) -> FxHashSet<&'a str> {
    let body = checker.body();
    let skipped: Vec<TextRange> = body
        .iter()
        .filter_map(Stmt::as_if_stmt)
        .filter_map(|stmt| {
            if is_main_test(&stmt.test) {
                Some(stmt.range)
            } else {
                is_type_checking_name(&stmt.test).then(|| {
                    TextRange::new(stmt.test.end(), stmt.elif_else_clauses.first().map_or(stmt.end(), Ranged::start))
                })
            }
        })
        .collect();
    let eager = eager_annotations(checker);
    let mut calls = Calls::default();
    calls.visit_body(body);
    let definitions = Definitions::new(body);
    let semantic = checker.semantic();
    let at_load = |reference: &Reference, invoked: &FxHashSet<TextRange>| {
        let runs = reference.function.map_or_else(
            || !skipped.iter().any(|range| range.contains_range(reference.range)),
            |function| invoked.contains(&function),
        );
        runs && match reference.evaluation {
            Evaluation::Runtime => true,
            Evaluation::Deferred => reference.function.is_some(),
            Evaluation::Annotation => eager.iter().any(|range| range.contains_range(reference.range)),
        }
    };
    // A lambda outside any function may run at once, as an argument of `sorted` or `map` does.
    let mut invoked: FxHashSet<TextRange> = calls.lambdas.iter().copied().collect();
    loop {
        let known = invoked.len();
        let started: Vec<TextRange> = semantic
            .references()
            .filter(|reference| at_load(reference, &invoked))
            .flat_map(|reference| {
                let range = reference.range;
                let trigger = if calls.called.contains(&range) {
                    Some(Trigger::Call)
                } else if calls.subclassed.contains(&range) {
                    Some(Trigger::Subclass)
                } else if calls.metaclasses.contains(&range) {
                    Some(Trigger::Metaclass)
                } else {
                    None
                };
                trigger.map(|trigger| definitions.run_by(reference.name, trigger)).unwrap_or_default()
            })
            .collect();
        invoked.extend(started);
        if invoked.len() == known {
            break;
        }
    }
    semantic.references().filter(|reference| at_load(reference, &invoked)).map(|reference| reference.name).collect()
}

// Some decorators read the annotations of what they decorate while the module loads, as `@dataclass` reads its
// fields.
fn eager_annotations(checker: &Checker) -> Vec<TextRange> {
    let semantic = checker.semantic();
    let signatures = checker
        .function_defs()
        .iter()
        .filter(|function| semantic.reads_annotations(&function.decorator_list))
        .flat_map(|function| signature_annotations(function).map(Ranged::range));
    let fields =
        checker.class_defs().iter().filter(|class| semantic.reads_annotations(&class.decorator_list)).flat_map(
            |class| class.body.iter().filter_map(Stmt::as_ann_assign_stmt).map(|field| field.annotation.range()),
        );
    signatures.chain(fields).collect()
}

#[derive(Clone, Copy)]
enum Trigger {
    Call,
    Subclass,
    Metaclass,
}

struct Class<'a> {
    methods: Vec<(&'a str, TextRange)>,
    bases: Vec<&'a str>,
}

struct Definitions<'a> {
    functions: FxHashMap<&'a str, TextRange>,
    classes: FxHashMap<&'a str, Class<'a>>,
}

impl<'a> Definitions<'a> {
    fn new(body: &'a [Stmt]) -> Self {
        let mut functions = FxHashMap::default();
        let mut classes = FxHashMap::default();
        for stmt in body {
            match stmt {
                Stmt::FunctionDef(function) => {
                    functions.insert(function.name.as_str(), function.range);
                }
                Stmt::ClassDef(class) => {
                    let methods = class
                        .body
                        .iter()
                        .filter_map(Stmt::as_function_def_stmt)
                        .map(|method| (method.name.as_str(), method.range))
                        .collect();
                    let bases =
                        class.bases().iter().filter_map(Expr::as_name_expr).map(|base| base.id.as_str()).collect();
                    classes.insert(class.name.as_str(), Class { methods, bases });
                }
                _ => {}
            }
        }
        Self { functions, classes }
    }

    // Construction can call inherited methods. Subclass creation can invoke an inherited `__init_subclass__`.
    fn run_by(&self, name: &str, trigger: Trigger) -> Vec<TextRange> {
        if let (Trigger::Call, Some(function)) = (trigger, self.functions.get(name)) {
            return vec![*function];
        }
        let mut ranges = Vec::new();
        let mut seen = FxHashSet::default();
        let mut pending = vec![name];
        while let Some(name) = pending.pop() {
            let Some(class) = self.classes.get(name).filter(|_| seen.insert(name)) else {
                continue;
            };
            ranges.extend(
                class
                    .methods
                    .iter()
                    .filter(|(method, _)| !matches!(trigger, Trigger::Subclass) || *method == "__init_subclass__")
                    .map(|(_, range)| *range),
            );
            pending.extend(&class.bases);
        }
        ranges
    }
}

#[derive(Default)]
struct Calls {
    called: FxHashSet<TextRange>,
    subclassed: FxHashSet<TextRange>,
    metaclasses: FxHashSet<TextRange>,
    lambdas: Vec<TextRange>,
    in_function: bool,
}

impl<'a> Visitor<'a> for Calls {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                self.called.extend(names(function.decorator_list.iter().map(|decorator| &decorator.expression)));
                let in_function = std::mem::replace(&mut self.in_function, true);
                walk_stmt(self, stmt);
                self.in_function = in_function;
                return;
            }
            Stmt::ClassDef(class) => {
                self.called.extend(names(class.decorator_list.iter().map(|decorator| &decorator.expression)));
                self.subclassed.extend(names(class.bases().iter()));
                self.metaclasses.extend(names(
                    class
                        .keywords()
                        .iter()
                        .filter(|keyword| keyword.arg.as_deref() == Some("metaclass"))
                        .map(|keyword| &keyword.value),
                ));
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Call(ast::ExprCall { func, .. }) => self.called.extend(names([func.as_ref()])),
            Expr::Lambda(lambda) if !self.in_function => {
                self.lambdas.push(lambda.range);
                let in_function = std::mem::replace(&mut self.in_function, true);
                walk_expr(self, expr);
                self.in_function = in_function;
                return;
            }
            _ => {}
        }
        walk_expr(self, expr);
    }
}

fn names<'a>(exprs: impl IntoIterator<Item = &'a Expr>) -> impl Iterator<Item = TextRange> {
    exprs.into_iter().filter_map(Expr::as_name_expr).map(Ranged::range)
}
