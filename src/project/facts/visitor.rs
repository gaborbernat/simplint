use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use ruff_python_ast::ExprContext;
use ruff_python_ast::helpers::resolve_imported_module_path;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_stdlib::identifiers::is_identifier;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use super::caught::class_argument;
use super::doctests::doctest_names;
use super::fixtures::is_pytest_plugins;
use super::loaded::{Globals, lexically_normalized};
use super::patches::named_attribute_call;
use super::runtime::{Constants, Runs, catches_import_error};
use super::{Channel, Channels, ImportSite, ModuleNames, Parts};
use crate::syntax::{alias_binding, attribute_call, last_segment, target_names};

// A name a `from module import name as bound` statement binds; `redundant` marks `name as name`, which the typing spec
// reads as a re-export.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(crate) struct ImportBinding {
    pub(crate) module: String,
    pub(crate) name: String,
    pub(crate) bound: String,
    pub(crate) redundant: bool,
}

pub(super) fn top_level_bindings(from_imports: &[FromImport]) -> Vec<ImportBinding> {
    from_imports
        .iter()
        // A `__future__` import binds a compiler flag, which no importer reads as a name of the module.
        .filter(|import| import.top_level && import.module != "__future__")
        .flat_map(|import| {
            import.names.iter().filter(|alias| alias.name.as_str() != "*").map(|alias| ImportBinding {
                module: import.module.clone(),
                name: alias.name.to_string(),
                bound: alias_binding(alias).to_owned(),
                redundant: alias.asname.as_ref().is_some_and(|asname| asname.as_str() == alias.name.as_str()),
            })
        })
        .collect()
}

// A `from module import ...` statement, with the module resolved against the file's module path, and whether it runs
// when the module loads, as `module_statements` reads that.
pub(crate) struct FromImport<'a> {
    pub(crate) module: String,
    pub(crate) names: &'a [ast::Alias],
    top_level: bool,
}

pub(super) fn grouped(names: BTreeMap<String, BTreeSet<String>>) -> Vec<ModuleNames> {
    names.into_iter().map(|(module, names)| (module, names.into_iter().collect())).collect()
}

// The value each name a file binds holds. A name bound again to a different value holds `None`, so no read through the
// name reads that value.
#[derive(Default)]
struct Bound<'a, T>(FxHashMap<&'a str, Option<T>>);

impl<'a, T: Clone + PartialEq> Bound<'a, T> {
    fn bind(&mut self, name: &'a str, value: Option<&T>) {
        let slot = self.0.entry(name).or_insert_with(|| value.cloned());
        if slot.as_ref() != value {
            *slot = None;
        }
    }

    fn get(&self, name: &str) -> Option<&T> {
        self.0.get(name)?.as_ref()
    }
}

// The kind of body a statement sits in. A class body binds class attributes, which other code reads through the class.
#[derive(Clone, Copy, PartialEq)]
enum Body {
    Class,
    Other,
}

// A function body, or the module outside them: the names it loads as values, and the modules its imports bind to a name,
// which count once the body is read and loads that name as a value. Its parameters also shadow the modules the names
// around it load and the loop bindings, which come back when the body is left.
#[derive(Default)]
pub(super) struct Frame<'a> {
    loads: FxHashSet<&'a str>,
    imports: Vec<(String, String)>,
    loaded: FxHashMap<&'a str, BTreeSet<String>>,
    shadowed: Vec<(&'a str, Option<Vec<String>>)>,
}

impl<'a> Frame<'a> {
    pub(super) fn bind(&mut self, name: &'a str, modules: &[String]) {
        self.loaded.entry(name).or_default().extend(modules.iter().cloned());
    }
}

pub(super) struct FactVisitor<'a> {
    pub(super) module_path: Option<&'a [String]>,
    pub(super) aliases: FxHashMap<&'a str, String>,
    pub(super) names: Channels<BTreeMap<String, BTreeSet<String>>>,
    pub(super) assigned_attributes: Vec<String>,
    // The names a `<receiver>.setattr` call sets on each module, which count only when the module is a file of the run.
    pub(super) module_writes: BTreeMap<String, BTreeSet<String>>,
    // The modules a write under a computed name may reach, as `pkg.report` for `setattr(report, name, value)`.
    pub(super) computed_writes: BTreeSet<String>,
    pub(super) base_classes: Vec<String>,
    pub(super) stars: BTreeSet<String>,
    pub(super) from_statements: Vec<FromImport<'a>>,
    // Whether the statement visited runs when the module loads: it sits at module level, or in a top-level `if` or
    // `try` block.
    pub(super) top_level: bool,
    pub(super) reads_docstrings: bool,
    // Whether the file reads a class's name off an object, as `type(obj).__name__` does, so renaming a class changes
    // it.
    pub(super) reads_class_names: bool,
    // The names whose `__name__` or `__qualname__` the file reads, as `helper` for `helper.__name__` or `Item` for
    // `pkg.Item.__qualname__`, which renaming the function or class changes.
    pub(super) reflected_names: BTreeSet<String>,
    // `import pkg.module as name` at the top level, as the name and the dotted module.
    pub(super) module_aliases: Vec<(String, String)>,
    // `import pkg.module` at the top level, by the root name it binds, which is the module that name stands for too.
    pub(super) plain_modules: Vec<String>,
    // The module the file is, so a bare class name it uses that no import binds names one of its own classes.
    own_module: String,
    pub(super) caught: Vec<(String, String)>,
    // The module-level names bound to a tuple, with the classes each of its elements names, as (module, name).
    pub(super) class_tuples: Vec<(String, Vec<(String, String)>)>,
    pub(super) fixture_requests: BTreeSet<String>,
    pub(super) pytest_requests: BTreeSet<String>,
    // The strings that name a fixture in its own decorator.
    fixture_names: FxHashSet<TextRange>,
    pub(super) doctest_names: BTreeSet<String>,
    // How many exception handlers the visit is inside.
    handler_depth: usize,
    // Whether the statement visited runs when the module loads. It does not inside a function, a lazy import, a branch
    // whose test is false at run time, such as `if TYPE_CHECKING:` or a `__main__` guard, or an `except ImportError:`
    // fallback, which runs only when the import it stands in for fails.
    runtime: Runs,
    // The names bound to a spec, as `spec_from_file_location(name, path)` builds, mapped to the module its path names,
    // which `module_from_spec` loads.
    pub(super) specs: FxHashMap<&'a str, Option<String>>,
    // The names bound to a module no static path resolves to a project module, which read names of any module as a
    // computed import does.
    pub(super) unknown_modules: FxHashSet<String>,
    // The names bound to the globals `runpy` returns, which a string key reads a name of.
    pub(super) globals: FxHashMap<&'a str, Globals>,
    // The string keys read off globals no static path names, which may be names of any module.
    pub(super) unresolved_globals: BTreeSet<String>,
    // The names bound to a tuple, list or set of string literals, as rows of strings, one per item; a name bound to
    // anything else too maps to `None`.
    string_rows: Bound<'a, Vec<Vec<String>>>,
    // The names a `for` over string literals binds, with the strings each takes in turn.
    pub(super) loop_strings: FxHashMap<&'a str, Vec<String>>,
    // The names a `for` over a literal of modules binds, with the module each item imports or names in turn.
    pub(super) loop_modules: FxHashMap<&'a str, Vec<String>>,
    // The file being visited and the directory its module path is relative to, both normalized, which the paths a
    // spec loads resolve against.
    pub(super) file: PathBuf,
    pub(super) root: Option<PathBuf>,
    // The module-level names bound once to a constant, as `_false = 0` or `DEBUG = True` is.
    pub(super) constants: Constants<'a>,
    pub(super) runtime_sites: Vec<ImportSite>,
    // The names the file loads other than as the object of an attribute, as `fast` in `codec = fast` is.
    pub(super) value_loads: FxHashSet<&'a str>,
    // The dotted names the file loads other than as the object of an attribute, as `app.fast` in `codec = app.fast` is.
    pub(super) dotted_loads: FxHashSet<String>,
    // The modules a class body's imports bind, which other code reads through the class, and the modules a function
    // body's imports bind whose names the body loads as values.
    pub(super) nested_modules: Vec<String>,
    // The kind of body the statements visited are directly in.
    body: Body,
    // The loads and pending imports of the function bodies being visited, innermost last.
    scopes: Vec<Frame<'a>>,
    // The loads and pending imports of the module's own statements outside any function or class body.
    module_scope: Frame<'a>,
    // The ranges of the objects of the attributes the file reads, which are not loads as a value.
    attribute_values: FxHashSet<TextRange>,
    // The facts only some rules read that the run collects.
    pub(super) parts: Parts,
}

impl<'a> FactVisitor<'a> {
    pub(super) fn new(
        file: &Path,
        module_path: Option<&'a [String]>,
        own_module: String,
        parts: Parts,
        constants: Constants<'a>,
    ) -> Self {
        let file = lexically_normalized(file);
        let root =
            module_path.and_then(|module_path| file.ancestors().nth(module_path.len())).map(lexically_normalized);
        Self {
            module_path,
            aliases: FxHashMap::default(),
            names: Channels::default(),
            assigned_attributes: Vec::new(),
            module_writes: BTreeMap::new(),
            computed_writes: BTreeSet::new(),
            base_classes: Vec::new(),
            stars: BTreeSet::new(),
            from_statements: Vec::new(),
            top_level: true,
            reads_docstrings: false,
            reads_class_names: false,
            reflected_names: BTreeSet::new(),
            module_aliases: Vec::new(),
            plain_modules: Vec::new(),
            own_module,
            caught: Vec::new(),
            class_tuples: Vec::new(),
            fixture_requests: BTreeSet::new(),
            pytest_requests: BTreeSet::new(),
            fixture_names: FxHashSet::default(),
            doctest_names: BTreeSet::new(),
            handler_depth: 0,
            runtime: Runs::Always,
            constants,
            specs: FxHashMap::default(),
            unknown_modules: FxHashSet::default(),
            globals: FxHashMap::default(),
            unresolved_globals: BTreeSet::new(),
            string_rows: Bound::default(),
            loop_strings: FxHashMap::default(),
            loop_modules: FxHashMap::default(),
            file,
            root,
            parts,
            runtime_sites: Vec::new(),
            value_loads: FxHashSet::default(),
            dotted_loads: FxHashSet::default(),
            nested_modules: Vec::new(),
            body: Body::Other,
            scopes: Vec::new(),
            module_scope: Frame::default(),
            attribute_values: FxHashSet::default(),
        }
    }
}

impl<'a> FactVisitor<'a> {
    // The object of an attribute is read through the attribute's name, which an import shows, so only other loads count
    // as values.
    fn record_value_load(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(ast::ExprAttribute { value, ctx, .. }) = expr {
            if ctx.is_load()
                && !self.attribute_values.contains(&expr.range())
                && let Some(name) = UnqualifiedName::from_expr(expr)
            {
                self.dotted_loads.insert(name.segments().join("."));
            }
            self.attribute_values.insert(value.range());
        } else if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, range, .. }) = expr
            && !self.attribute_values.contains(range)
        {
            self.value_loads.insert(id.as_str());
            self.scope_mut().loads.insert(id.as_str());
        }
    }

    // The innermost function body being visited, or the module outside them.
    pub(super) fn scope_mut(&mut self) -> &mut Frame<'a> {
        self.scopes.last_mut().unwrap_or(&mut self.module_scope)
    }

    // The modules the innermost scope that binds the name loads, which a name bound to no module loads none of.
    pub(super) fn loaded_named(&self, name: &str) -> Vec<String> {
        let mut scopes = self.scopes.iter().rev().chain(std::iter::once(&self.module_scope));
        scopes
            .find_map(|scope| scope.loaded.get(name))
            .map_or_else(Vec::new, |modules| modules.iter().cloned().collect())
    }

    // Walks a function or class statement, whose statements sit in the body it names.
    fn walk_body(&mut self, stmt: &'a Stmt, body: Body) {
        let outer = std::mem::replace(&mut self.body, body);
        visitor::walk_stmt(self, stmt);
        self.body = outer;
    }

    // Enters a function body, whose parameters shadow the loop bindings and the modules of the names around them.
    fn enter_function(&mut self, parameters: Vec<&'a str>) {
        let shadowed = self.shadow(parameters.clone());
        let loaded = parameters.into_iter().map(|name| (name, BTreeSet::new())).collect();
        self.scopes.push(Frame { loaded, shadowed, ..Frame::default() });
    }

    // Ends the innermost function body: its imports count for the names it loads, and its loads reach the scope it sits
    // in, since a nested function reads the names of the one around it.
    fn leave_function(&mut self) {
        let frame = self.scopes.pop().unwrap_or_default();
        settle(&frame, &mut self.nested_modules);
        restore_bindings(&mut self.loop_modules, frame.shadowed);
        self.scope_mut().loads.extend(frame.loads);
    }

    // Ends the module: the imports outside any function body count for the names the module loads.
    pub(super) fn finish_module(&mut self) {
        let frame = std::mem::take(&mut self.module_scope);
        settle(&frame, &mut self.nested_modules);
    }

    // The module an import in a function or class body binds to a name, as `fast` for `from pkg import fast` in a class.
    fn record_nested(&mut self, top_level: bool, bound: &str, module: &str) {
        if top_level {
            return;
        }
        if self.body == Body::Class {
            self.nested_modules.push(module.to_owned());
        } else {
            self.scope_mut().imports.push((bound.to_owned(), module.to_owned()));
        }
    }

    // Records what a `from` import binds and the import sites the index follows.
    fn import_from(&mut self, stmt: &'a Stmt, import: &'a ast::StmtImportFrom, top_level: bool) {
        let ast::StmtImportFrom { module, names, level, is_lazy, .. } = import;
        let runtime = self.runtime;
        let runs = runtime != Runs::Never;
        let module = module.as_ref().map(ast::Identifier::as_str);
        if let Some(resolved) = resolve_imported_module_path(*level, module, self.module_path).map(Cow::into_owned) {
            let star = names.iter().any(|alias| alias.name.as_str() == "*");
            if star {
                self.stars.insert(resolved.clone());
            }
            if runs && !is_lazy && self.parts.has(Parts::RUNTIME_IMPORTS) {
                let submodules = names
                    .iter()
                    .filter(|alias| alias.name.as_str() != "*")
                    .map(|alias| format!("{resolved}.{}", alias.name.as_str()))
                    .collect();
                self.runtime_sites.push(ImportSite::new(resolved.clone(), submodules, stmt.range(), runtime));
            }
            for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                // A test module that imports a fixture shares it with its tests, which request it by name.
                if self.parts.has(Parts::FIXTURES) {
                    self.fixture_requests.insert(alias.name.to_string());
                }
                self.names[Channel::Import].entry(resolved.clone()).or_default().insert(alias.name.as_str().to_owned());
                self.aliases.insert(alias_binding(alias), format!("{resolved}.{}", alias.name.as_str()));
            }
            for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                self.record_nested(top_level, alias_binding(alias), &format!("{resolved}.{}", alias.name.as_str()));
            }
            self.from_statements.push(FromImport { module: resolved, names, top_level });
        }
    }
}

impl<'a> FactVisitor<'a> {
    // A module-level name a plain assignment sets to a class, as `Base = Release`, names that class from then on, so a
    // base class written as `Base` is the class `Release`.
    fn bind_class_alias(&mut self, targets: &'a [Expr], value: &'a Expr) {
        let [Expr::Name(target)] = targets else {
            return;
        };
        if !self.top_level {
            return;
        }
        let Some((module, name)) = self.class_path(value) else {
            return;
        };
        if name.starts_with(|first: char| first.is_ascii_uppercase()) {
            self.aliases.insert(target.id.as_str(), format!("{module}.{name}"));
        }
    }

    // Binds each name a plain assignment sets to the string rows its value holds, or unbinds it when another value sets it.
    fn bind_string_rows(&mut self, targets: &'a [Expr], value: &'a Expr) {
        let rows = self.string_rows(value);
        for name in targets.iter().filter_map(Expr::as_name_expr) {
            self.string_rows.bind(name.id.as_str(), rows.as_ref());
        }
    }

    // The rows of a tuple, list or set of string literals, or of the rows a name bound to one holds.
    fn string_rows(&self, expr: &Expr) -> Option<Vec<Vec<String>>> {
        match expr {
            Expr::Name(name) => self.string_rows.get(name.id.as_str()).cloned(),
            Expr::Tuple(ast::ExprTuple { elts, .. })
            | Expr::List(ast::ExprList { elts, .. })
            | Expr::Set(ast::ExprSet { elts, .. }) => elts.iter().map(string_row).collect(),
            _ => None,
        }
    }

    // The names a `for` over string rows binds, each with the strings it takes: `name` for `for name in ("a", "b")`,
    // and `name` at its position for `for name, _ in (("a", 1), ...)`. A loop over anything else binds none.
    fn loop_bindings(&self, target: &'a Expr, iter: &Expr) -> Vec<(&'a str, Vec<String>)> {
        let Some(rows) = self.string_rows(iter) else {
            return Vec::new();
        };
        match target {
            Expr::Name(name) if rows.iter().all(|row| row.len() == 1) => {
                vec![(name.id.as_str(), rows.into_iter().flatten().collect())]
            }
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. })
                if rows.iter().all(|row| row.len() == elts.len()) =>
            {
                elts.iter()
                    .enumerate()
                    .filter_map(|(index, elt)| {
                        let name = elt.as_name_expr()?;
                        Some((name.id.as_str(), rows.iter().map(|row| row[index].clone()).collect()))
                    })
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    // The strings and modules a loop's name takes in turn hold for its body, and the names' earlier values come back after.
    fn visit_for(&mut self, stmt: &'a Stmt, target: &'a Expr, iter: &Expr) {
        let bound = self.loop_bindings(target, iter);
        let modules = self.loop_modules(target, iter);
        let previous = previous_bindings(&self.loop_strings, &bound);
        let previous_modules = previous_bindings(&self.loop_modules, &modules);
        self.loop_strings.extend(bound);
        self.loop_modules.extend(modules);
        visitor::walk_stmt(self, stmt);
        restore_bindings(&mut self.loop_strings, previous);
        restore_bindings(&mut self.loop_modules, previous_modules);
    }

    // The loop mappings of the names a function or lambda parameter binds, taken out for the body the parameters shadow.
    fn shadow(&mut self, names: Vec<&'a str>) -> Vec<(&'a str, Option<Vec<String>>)> {
        names.into_iter().map(|name| (name, self.loop_modules.remove(name))).collect()
    }

    // A `for` over a literal tuple or list of modules binds its name to the module each item imports or names, in turn.
    // An item the run cannot read as a module contributes none.
    fn loop_modules(&self, target: &'a Expr, iter: &Expr) -> Vec<(&'a str, Vec<String>)> {
        let (Expr::Name(name), Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. })) =
            (target, iter)
        else {
            return Vec::new();
        };
        vec![(name.id.as_str(), elts.iter().filter_map(|elt| self.module_of(elt)).collect())]
    }

    // The module an item names: the one an `importlib.import_module("name")` call imports, or the one a name or dotted
    // path an import binds stands for.
    fn module_of(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Call(ast::ExprCall { func, arguments, .. }) if last_segment(func) == Some("import_module") => {
                arguments.args.first()?.as_string_literal_expr().map(|module| module.value.to_str().to_owned())
            }
            _ => self.qualified(expr),
        }
    }
}

// The strings of a tuple, list or set item of string literals: the item itself, or the elements of a tuple or list.
fn string_row(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::StringLiteral(literal) => Some(vec![literal.value.to_str().to_owned()]),
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => elts
            .iter()
            .map(|elt| elt.as_string_literal_expr().map(|literal| literal.value.to_str().to_owned()))
            .collect(),
        _ => None,
    }
}

impl<'a> Visitor<'a> for FactVisitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        for name in bound_names(stmt) {
            self.loop_modules.remove(name);
        }
        if let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt {
            self.bind_loaded_module(targets, value);
            self.bind_globals(targets, value);
            self.bind_string_rows(targets, value);
            self.bind_class_alias(targets, value);
        }
        self.bind_class_tuple(stmt);
        let top_level = self.top_level;
        let runtime = self.runtime;
        self.top_level &= matches!(stmt, Stmt::If(_) | Stmt::Try(_));
        let runs = runtime != Runs::Never;
        match stmt {
            Stmt::FunctionDef(function) => {
                if self.parts.has(Parts::FIXTURES) {
                    self.request_parameters(function);
                }
                self.runtime = Runs::Never;
                self.enter_function(parameter_names(&function.parameters));
                self.walk_body(stmt, Body::Other);
                self.leave_function();
            }
            Stmt::ImportFrom(import) => self.import_from(stmt, import, top_level),
            Stmt::Import(ast::StmtImport { names, is_lazy, .. }) => {
                for alias in names {
                    if runs && !is_lazy && self.parts.has(Parts::RUNTIME_IMPORTS) {
                        let module = alias.name.as_str().to_owned();
                        self.runtime_sites.push(ImportSite::new(module, Vec::new(), stmt.range(), runtime));
                    }
                    let full = alias.name.as_str();
                    if let Some(asname) = &alias.asname {
                        if top_level {
                            self.module_aliases.push((asname.as_str().to_owned(), full.to_owned()));
                        }
                        self.record_nested(top_level, asname.as_str(), full);
                        self.aliases.insert(asname.as_str(), full.to_owned())
                    } else {
                        let root = full.split('.').next().unwrap_or_default();
                        if top_level {
                            self.plain_modules.push(root.to_owned());
                        }
                        self.record_nested(top_level, root, full);
                        self.aliases.insert(root, root.to_owned())
                    };
                }
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. })
                if self.parts.has(Parts::FIXTURES) && targets.iter().any(is_pytest_plugins) =>
            {
                self.load_plugins(value);
                visitor::walk_stmt(self, stmt);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. })
                if self.parts.has(Parts::FIXTURES) && is_pytest_plugins(target) =>
            {
                self.load_plugins(value);
                visitor::walk_stmt(self, stmt);
            }
            Stmt::ClassDef(class) => {
                self.record_bases(class);
                self.walk_body(stmt, Body::Class);
            }
            Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) => {
                // A clause runs when its own test may hold and every test before it may fail.
                let clauses = std::iter::once((Some(&**test), body))
                    .chain(elif_else_clauses.iter().map(|clause| (clause.test.as_ref(), &clause.body)));
                let mut reached = runtime;
                for (test, body) in clauses {
                    let (holds, fails) = test.map_or((Runs::Always, Runs::Never), |test| {
                        self.visit_expr(test);
                        self.branches(test)
                    });
                    self.runtime = reached.and(holds);
                    self.visit_body(body);
                    reached = reached.and(fails);
                }
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                self.visit_body(body);
                for handler in handlers {
                    let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { type_, .. }) = handler;
                    if type_.as_deref().is_some_and(catches_import_error) {
                        self.runtime = Runs::Never;
                    }
                    self.visit_except_handler(handler);
                    self.runtime = runtime;
                }
                self.visit_body(orelse);
                self.visit_body(finalbody);
            }
            Stmt::For(ast::StmtFor { target, iter, .. }) => self.visit_for(stmt, target, iter),
            _ => visitor::walk_stmt(self, stmt),
        }
        self.runtime = runtime;
        self.top_level = top_level;
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { type_, .. }) = handler;
        if let Some(type_) = type_ {
            self.catch(type_);
        }
        self.handler_depth += 1;
        visitor::walk_except_handler(self, handler);
        self.handler_depth -= 1;
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        if let ast::Pattern::MatchClass(ast::PatternMatchClass { cls, .. }) = pattern
            && self.handler_depth > 0
        {
            self.catch(cls);
        }
        visitor::walk_pattern(self, pattern);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let shadowed = match expr {
            Expr::Lambda(ast::ExprLambda { parameters: Some(parameters), .. }) => {
                self.shadow(parameter_names(parameters))
            }
            _ => Vec::new(),
        };
        self.reads_docstrings |= reads_docstring(expr);
        if self.parts.has(Parts::REFLECTED_NAMES) {
            self.reads_class_names |= reads_class_name(expr);
            if let Some(name) = reflected_name(expr) {
                self.reflected_names.insert(name.to_owned());
            }
        }
        if let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr
            && self.handler_depth > 0
            && let Some(classes) = class_argument(func, arguments)
        {
            self.catch(classes);
        }
        self.catch_suppressed(expr);
        self.record_value_load(expr);
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.assigned_attributes.push(attr.as_str().to_owned());
                // A store on a module object reaches the module's name as `setattr` does, so the module's own name
                // stays a module attribute and is not localized or renamed as one nothing reads.
                if let Some(module) = self.qualified(value) {
                    self.record_references(module, [attr.as_str().to_owned()]);
                }
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                // A read off a loaded module is an import of the name, as `module.name` after `import module` is.
                for module in self.loaded_modules_of(value) {
                    self.names[Channel::Import].entry(module).or_default().insert(attr.as_str().to_owned());
                }
                // `lazy_fixture(db.__name__)` and `usefixtures(db.__name__)` request a fixture by its function's name.
                if attr.as_str() == "__name__"
                    && let Some(name) = value.as_name_expr()
                    && self.parts.has(Parts::FIXTURES)
                {
                    self.fixture_requests.insert(name.id.to_string());
                }
                if let Some(path) = self.qualified(expr) {
                    Self::record_path(&mut self.names[Channel::Import], &path);
                    return;
                }
            }
            Expr::StringLiteral(literal) => {
                let text = literal.value.to_str();
                if self.parts.has(Parts::DOCTESTS) && text.contains(">>>") {
                    self.doctest_names.extend(doctest_names(text));
                    self.doctests(text);
                }
                // A string that spells a name may request a fixture: `usefixtures("db")`, `getfixturevalue("db")`, or
                // `getfixturevalue(request.param)` over `params=["sqlite", "postgres"]`.
                if self.parts.has(Parts::FIXTURES)
                    && is_identifier(text)
                    && !self.fixture_names.contains(&literal.range)
                {
                    self.fixture_requests.insert(text.to_owned());
                }
                if text.contains('.') && text.split('.').all(is_identifier) {
                    Self::record_path(&mut self.names[Channel::Reference], text);
                }
            }
            Expr::Subscript(ast::ExprSubscript { value, slice, ctx: ExprContext::Load, .. }) => {
                if let Some(key) = slice.as_string_literal_expr() {
                    self.read_globals(value, key.value.to_str());
                }
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                self.record_named_handler(func, arguments);
                // The name `@pytest.fixture(name="db")` gives is the fixture's own, not a request of it.
                if self.parts.has(Parts::FIXTURES)
                    && last_segment(func) == Some("fixture")
                    && let Some(name) = arguments.find_keyword("name")
                {
                    self.fixture_names.insert(name.value.range());
                }
                let named = self.named_names(arguments);
                if let [_, Expr::StringLiteral(attribute), ..] = &*arguments.args
                    && func.as_name_expr().is_some_and(|name| name.id.as_str() == "setattr")
                {
                    self.assigned_attributes.push(attribute.value.to_str().to_owned());
                }
                // A name a call sets through a literal or a loop over string literals reads its module's attribute, as
                // `setattr(module, name)` does for each string.
                if named_attribute_call(func) || matches!(arguments.args.get(1), Some(Expr::StringLiteral(_))) {
                    for (module, names) in &named {
                        self.record_references(module.clone(), names.iter().cloned());
                    }
                }
                for (module, names) in Self::module_writes(func, arguments, named) {
                    self.module_writes.entry(module).or_default().extend(names);
                }
                if self.parts.has(Parts::COMPUTED_WRITES) {
                    self.computed_write(func, arguments);
                }
                // `runpy.run_path(path)["name"]` and `.get("name")` read a name of the module the globals run.
                if let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = func.as_ref()
                    && attr.as_str() == "get"
                    && let Some(Expr::StringLiteral(key)) = arguments.args.first()
                {
                    self.read_globals(value, key.value.to_str());
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
        restore_bindings(&mut self.loop_modules, shadowed);
    }
}

impl FactVisitor<'_> {
    fn record_named_handler(&mut self, func: &Expr, arguments: &ast::Arguments) {
        let target = if self.qualified(func).as_deref() == Some("pkgutil.resolve_name") {
            arguments.args.first().and_then(Expr::as_string_literal_expr)
        } else if func.as_attribute_expr().is_some_and(|attribute| attribute.attr.as_str() == "set_defaults") {
            arguments.find_keyword("handler").and_then(|keyword| keyword.value.as_string_literal_expr())
        } else {
            None
        };
        if let Some((module, name)) = target.and_then(|literal| module_attribute(literal.value.to_str())) {
            self.names[Channel::Import].entry(module.to_owned()).or_default().insert(name.to_owned());
            self.record_references(module.to_owned(), [name.to_owned()]);
        }
    }
}

fn module_attribute(path: &str) -> Option<(&str, &str)> {
    let (module, attribute) = path.split_once(':')?;
    (module.split('.').all(is_identifier) && attribute.split('.').all(is_identifier))
        .then(|| (module, attribute.split('.').next().expect("an identifier")))
}

// The names a loop binds, paired with the value each of them held before it, so the loop can hand them back.
fn previous_bindings<'a>(
    bindings: &FxHashMap<&'a str, Vec<String>>,
    bound: &[(&'a str, Vec<String>)],
) -> Vec<(&'a str, Option<Vec<String>>)> {
    bound.iter().map(|(name, _)| (*name, bindings.get(name).cloned())).collect()
}

// The modules a scope's imports bind under names the scope loads as values, which the file reads as nested modules.
fn settle(frame: &Frame, into: &mut Vec<String>) {
    into.extend(
        frame
            .imports
            .iter()
            .filter(|(bound, _)| frame.loads.contains(bound.as_str()))
            .map(|(_, module)| module.clone()),
    );
}

fn restore_bindings<'a>(bindings: &mut FxHashMap<&'a str, Vec<String>>, previous: Vec<(&'a str, Option<Vec<String>>)>) {
    for (name, values) in previous {
        match values {
            Some(values) => _ = bindings.insert(name, values),
            None => _ = bindings.remove(name),
        }
    }
}

// The names a statement binds in its own scope, which end a loop's mapping of a module under the name.
fn bound_names(stmt: &Stmt) -> Vec<&str> {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, .. }) => targets.iter().flat_map(target_names).collect(),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            target_names(target)
        }
        Stmt::For(ast::StmtFor { target, .. }) => target_names(target),
        Stmt::With(ast::StmtWith { items, .. }) => {
            items.iter().filter_map(|item| item.optional_vars.as_deref()).flat_map(target_names).collect()
        }
        Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => names
            .iter()
            .filter_map(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str().split('.').next())
            .collect(),
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            vec![name.as_str()]
        }
        _ => Vec::new(),
    }
}

fn parameter_names(parameters: &ast::Parameters) -> Vec<&str> {
    parameters.iter().map(|parameter| parameter.name().as_str()).collect()
}

// The dotted name `segments` stand for through the import that binds their head, such as `pkg.models.Partition` for
// `models.Partition` after `from pkg import models`.
pub(crate) fn qualified(segments: &[&str], aliases: &FxHashMap<&str, String>) -> Option<String> {
    let (first, rest) = segments.split_first()?;
    let module = aliases.get(first)?;
    Some(std::iter::once(module.as_str()).chain(rest.iter().copied()).collect::<Vec<_>>().join("."))
}

impl FactVisitor<'_> {
    pub(super) fn qualified(&self, expr: &Expr) -> Option<String> {
        qualified(UnqualifiedName::from_expr(expr)?.segments(), &self.aliases)
    }

    fn record_bases(&mut self, class: &ast::StmtClassDef) {
        let bases: Vec<String> = class.bases().iter().filter_map(|base| self.base_class(base)).collect();
        self.base_classes.extend(bases);
    }

    // The name a base class is known by, `Release` for `Base` in `from models import Release as Base`.
    fn base_class(&self, base: &Expr) -> Option<String> {
        let written = UnqualifiedName::from_expr(base)?;
        let bound = self.qualified(base).unwrap_or_else(|| written.segments().join("."));
        bound.rsplit('.').next().map(str::to_owned)
    }

    // The module and name a class expression names: the qualified name an import gives, or a bare name no import binds,
    // which is a class of this module.
    pub(super) fn class_path(&self, expr: &Expr) -> Option<(String, String)> {
        let name = UnqualifiedName::from_expr(expr)?;
        let segments = name.segments();
        match qualified(segments, &self.aliases) {
            Some(path) => path.rsplit_once('.').map(|(module, name)| (module.to_owned(), name.to_owned())),
            None => match segments {
                [name] => Some((self.own_module.clone(), (*name).to_owned())),
                _ => None,
            },
        }
    }

    // Records the names other code reaches on a module by a store or a call that names them, which a rename would break.
    pub(super) fn record_references(&mut self, module: String, names: impl IntoIterator<Item = String>) {
        self.names[Channel::Reference].entry(module).or_default().extend(names);
    }

    pub(super) fn record_path(names: &mut BTreeMap<String, BTreeSet<String>>, path: &str) {
        let parts: Vec<&str> = path.split('.').collect();
        for split in 1..parts.len() {
            names.entry(parts[..split].join(".")).or_default().insert(parts[split].to_owned());
        }
    }
}

// `obj.__doc__`, `getattr(obj, "__doc__")` or a call to a `getdoc`, such as `inspect.getdoc(obj)`; a bare `__doc__` is
// the module's own docstring, which no function or class docstring feeds.
fn reads_docstring(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(ast::ExprAttribute { attr, ctx: ExprContext::Load, .. }) => attr.as_str() == "__doc__",
        Expr::Call(ast::ExprCall { func, .. }) => {
            UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().last() == Some(&"getdoc"))
                || attribute_call(expr, &["getattr"]).is_some_and(|(_, name)| {
                    name.as_string_literal_expr().is_some_and(|text| text.value.to_str() == "__doc__")
                })
        }
        _ => false,
    }
}

// `type(obj).__name__` or `obj.__class__.__qualname__`, which read a class's name off an object.
fn reads_class_name(expr: &Expr) -> bool {
    let Expr::Attribute(ast::ExprAttribute { attr, value, ctx: ExprContext::Load, .. }) = expr else {
        return false;
    };
    matches!(attr.as_str(), "__name__" | "__qualname__")
        && match value.as_ref() {
            Expr::Call(ast::ExprCall { func, .. }) => {
                func.as_name_expr().is_some_and(|name| name.id.as_str() == "type")
            }
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => attr.as_str() == "__class__",
            // A classmethod reads its class's name off `cls`.
            Expr::Name(name) => name.id.as_str() == "cls",
            _ => false,
        }
}

// The name whose `__name__` or `__qualname__` an expression reads: the variable of `name.__name__`, or the attribute of
// `pkg.name.__qualname__`.
fn reflected_name(expr: &Expr) -> Option<&str> {
    let Expr::Attribute(ast::ExprAttribute { attr, value, ctx: ExprContext::Load, .. }) = expr else {
        return None;
    };
    if !matches!(attr.as_str(), "__name__" | "__qualname__") {
        return None;
    }
    match value.as_ref() {
        Expr::Name(name) => Some(name.id.as_str()),
        Expr::Attribute(ast::ExprAttribute { attr, .. }) => Some(attr.as_str()),
        _ => None,
    }
}
