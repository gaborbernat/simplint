use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use ruff_python_ast::ExprContext;
use ruff_python_ast::helpers::resolve_imported_module_path;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_stdlib::identifiers::is_identifier;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use super::runtime::{Runs, catches_import_error};
use super::{ImportSite, ModuleNames, Parts};
use crate::project::receivers::module_parts;
use crate::syntax::{attribute_call, has_decorator, last_segment};

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
                bound: alias.asname.as_ref().unwrap_or(&alias.name).to_string(),
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

// The modules each name loads, keyed by the scope that binds the name: the module's names, and those of each function
// the visit is inside, innermost last. A name a function binds shadows the module's, and a name bound to several
// modules reads each of them.
#[derive(Default)]
struct LoadedModules<'a> {
    module: FxHashMap<&'a str, BTreeSet<String>>,
    functions: Vec<FxHashMap<&'a str, BTreeSet<String>>>,
}

impl<'a> LoadedModules<'a> {
    fn bind(&mut self, name: &'a str, modules: &[String]) {
        let scope = self.functions.last_mut().unwrap_or(&mut self.module);
        scope.entry(name).or_default().extend(modules.iter().cloned());
    }

    // The modules the innermost scope that binds the name loads, which a name bound to no module loads none of.
    fn of(&self, name: &str) -> Vec<String> {
        let mut scopes = self.functions.iter().rev().chain(std::iter::once(&self.module));
        scopes.find_map(|scope| scope.get(name)).map_or_else(Vec::new, |modules| modules.iter().cloned().collect())
    }

    fn enter(&mut self, parameters: Vec<&'a str>) {
        self.functions.push(parameters.into_iter().map(|name| (name, BTreeSet::new())).collect());
    }
}

pub(super) struct FactVisitor<'a> {
    pub(super) module_path: Option<&'a [String]>,
    pub(super) aliases: FxHashMap<&'a str, String>,
    pub(super) edges: BTreeMap<String, BTreeSet<String>>,
    pub(super) references: BTreeMap<String, BTreeSet<String>>,
    pub(super) doctest_references: BTreeMap<String, BTreeSet<String>>,
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
    // `import pkg.module` at the top level, as the root name it binds and the module of that name.
    pub(super) plain_modules: Vec<(String, String)>,
    // The module the file is, so a bare class name it uses that no import binds names one of its own classes.
    pub(super) own_module: String,
    pub(super) caught: Vec<(String, String)>,
    pub(super) fixture_requests: BTreeSet<String>,
    pub(super) pytest_requests: BTreeSet<String>,
    // The strings that name a fixture in its own decorator.
    pub(super) fixture_names: FxHashSet<TextRange>,
    pub(super) doctest_names: BTreeSet<String>,
    // How many exception handlers the visit is inside.
    pub(super) handler_depth: usize,
    // Whether the statement visited runs when the module loads. It does not inside a function, a lazy import, a branch
    // whose test is false at run time, such as `if TYPE_CHECKING:` or a `__main__` guard, or an `except ImportError:`
    // fallback, which runs only when the import it stands in for fails.
    pub(super) runtime: Runs,
    // The names bound to a module a string-named import call loads, so `mod.name` reads a name of each of them.
    loaded_modules: LoadedModules<'a>,
    // The names bound to a spec, as `spec_from_file_location(name, path)` builds, mapped to the module its path names,
    // which `module_from_spec` loads.
    specs: FxHashMap<&'a str, Option<String>>,
    // The names bound to a module no static path resolves to a project module, which read names of any module as a
    // computed import does.
    pub(super) unknown_modules: FxHashSet<String>,
    // The names bound to the globals `runpy` returns, which a string key reads a name of.
    globals: FxHashMap<&'a str, Globals>,
    // The string keys read off globals no static path names, which may be names of any module.
    pub(super) unresolved_globals: BTreeSet<String>,
    // The names bound to a tuple, list or set of string literals, as rows of strings, one per item; a name bound to
    // anything else too maps to `None`.
    string_rows: Bound<'a, Vec<Vec<String>>>,
    // The names a `for` over string literals binds, with the strings each takes in turn.
    loop_strings: FxHashMap<&'a str, Vec<String>>,
    // The names a `for` over a literal of modules binds, with the module each item imports or names in turn.
    loop_modules: FxHashMap<&'a str, Vec<String>>,
    // The file being visited and the directory its module path is relative to, both normalized, which the paths a
    // spec loads resolve against.
    file: PathBuf,
    root: Option<PathBuf>,
    // The module-level names bound once to a false constant, as `_false = 0`, whose `if` bodies never run.
    pub(super) falsy_constants: FxHashSet<&'a str>,
    pub(super) runtime_sites: Vec<ImportSite>,
    // The names the file loads other than as the object of an attribute, as `fast` in `codec = fast` is.
    pub(super) value_loads: FxHashSet<&'a str>,
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
        falsy_constants: FxHashSet<&'a str>,
    ) -> Self {
        let file = lexically_normalized(file);
        let root =
            module_path.and_then(|module_path| file.ancestors().nth(module_path.len())).map(lexically_normalized);
        Self {
            module_path,
            aliases: FxHashMap::default(),
            edges: BTreeMap::new(),
            references: BTreeMap::new(),
            doctest_references: BTreeMap::new(),
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
            fixture_requests: BTreeSet::new(),
            pytest_requests: BTreeSet::new(),
            fixture_names: FxHashSet::default(),
            doctest_names: BTreeSet::new(),
            handler_depth: 0,
            runtime: Runs::Always,
            falsy_constants,
            loaded_modules: LoadedModules::default(),
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
            attribute_values: FxHashSet::default(),
        }
    }
}

impl<'a> FactVisitor<'a> {
    // The object of an attribute is read through the attribute's name, which an import shows, so only other loads count
    // as values.
    fn record_value_load(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(ast::ExprAttribute { value, .. }) = expr {
            self.attribute_values.insert(value.range());
        } else if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, range, .. }) = expr
            && !self.attribute_values.contains(range)
        {
            self.value_loads.insert(id.as_str());
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
                self.edges.entry(resolved.clone()).or_default().insert(alias.name.as_str().to_owned());
                let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                self.aliases.insert(bound, format!("{resolved}.{}", alias.name.as_str()));
            }
            self.from_statements.push(FromImport { module: resolved, names, top_level });
        }
    }
}

impl<'a> FactVisitor<'a> {
    // Binds each name a plain assignment sets to the modules its value loads, in the scope the assignment is in. A spec a
    // `spec_from_file_location` call builds is bound the same way, to the module its path names.
    fn bind_loaded_module(&mut self, targets: &'a [Expr], value: &'a Expr) {
        let modules = self.loaded_modules_of(value);
        let spec = self.spec_module(value);
        let unknown = modules.is_empty() && self.is_unknown_module(value);
        for name in targets.iter().filter_map(Expr::as_name_expr) {
            let bound = name.id.as_str();
            self.specs.insert(bound, spec.clone());
            if unknown {
                self.unknown_modules.insert(bound.to_owned());
            }
            self.loaded_modules.bind(bound, &modules);
        }
    }

    // The project modules an expression loads or names: a string-named import call, a module built from a spec, a
    // variable bound to modules, or a submodule read off the package an `__import__` call returns.
    fn loaded_modules_of(&self, expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Name(name) => self.loaded_modules.of(name.id.as_str()),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if is_dunder_import_chain(value) => {
                self.loaded_modules_of(value).into_iter().map(|module| format!("{module}.{attr}")).collect()
            }
            _ => loaded_module(expr).map(str::to_owned).or_else(|| self.module_from_spec(expr)).into_iter().collect(),
        }
    }

    // Binds each name a plain assignment sets to the globals its value returns, or unbinds it to unknown when another
    // value sets it.
    fn bind_globals(&mut self, targets: &'a [Expr], value: &'a Expr) {
        let globals = self.globals_of(value);
        for name in targets.iter().filter_map(Expr::as_name_expr) {
            let bound = name.id.as_str();
            match &globals {
                Some(globals) => {
                    let slot = self.globals.entry(bound).or_insert_with(|| globals.clone());
                    if *slot != *globals {
                        *slot = Globals::Unknown;
                    }
                }
                None => {
                    if let Some(slot) = self.globals.get_mut(bound) {
                        *slot = Globals::Unknown;
                    }
                }
            }
        }
    }

    // The globals a `runpy` call returns, as a dict of the module's names: `run_module("pkg.mod")` runs the module its
    // literal names, and `run_path(path)` the file its path names, which is unknown when no static path names a project
    // module. `None` for any other value, or for a name bound to none of these.
    fn globals_of(&self, expr: &Expr) -> Option<Globals> {
        match expr {
            Expr::Name(name) => self.globals.get(name.id.as_str()).cloned(),
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                let target = arguments.args.first()?;
                match self.qualified(func)?.as_str() {
                    "runpy.run_module" => Some(
                        target
                            .as_string_literal_expr()
                            .map_or(Globals::Unknown, |module| Globals::Module(module.value.to_str().to_owned())),
                    ),
                    "runpy.run_path" => Some(self.file_module(target).map_or(Globals::Unknown, Globals::Module)),
                    _ => None,
                }
            }
            _ => None,
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

    // Records a string key read off globals, which imports the name from the module the globals run, or may read any
    // module's name when no static path names it.
    fn read_globals(&mut self, value: &Expr, key: &str) {
        match self.globals_of(value) {
            Some(Globals::Module(module)) => _ = self.edges.entry(module).or_default().insert(key.to_owned()),
            Some(Globals::Unknown) => _ = self.unresolved_globals.insert(key.to_owned()),
            None => {}
        }
    }

    fn module_from_spec(&self, expr: &Expr) -> Option<String> {
        self.spec_module(module_from_spec_arg(expr)?)
    }

    // The project module a spec loads: the module its `spec_from_file_location` call's path names, or a variable bound
    // to such a spec. A path outside the search root, or one that is not static, names none.
    fn spec_module(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => self.specs.get(name.id.as_str()).cloned().flatten(),
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if last_segment(func) == Some("spec_from_file_location") =>
            {
                self.file_module(arguments.args.get(1)?)
            }
            _ => None,
        }
    }

    // Whether an expression loads a module no static path resolves, as `module_from_spec` of an unresolved spec does,
    // or names a variable bound to one.
    fn is_unknown_module(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => self.unknown_modules.contains(name.id.as_str()),
            _ => module_from_spec_arg(expr).is_some(),
        }
    }

    fn file_module(&self, expr: &Expr) -> Option<String> {
        let root = self.root.as_deref()?;
        let path = lexically_normalized(&root.join(self.static_path(expr)?));
        module_of(path.strip_prefix(root).ok()?)
    }

    // The path an expression spells out: `__file__`, a string literal, `Path(...)` or `os.path.join(...)` of such, with
    // `/`, `.parent`, `.parents[n]` and `os.path.dirname`. A relative path is relative to the search root.
    fn static_path(&self, expr: &Expr) -> Option<PathBuf> {
        match expr {
            Expr::Name(name) if name.id.as_str() == "__file__" => Some(self.file.clone()),
            Expr::StringLiteral(literal) => Some(PathBuf::from(literal.value.to_str())),
            Expr::BinOp(ast::ExprBinOp { left, op: ast::Operator::Div, right, .. }) => {
                Some(self.static_path(left)?.join(self.static_path(right)?))
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "parent" => {
                self.static_path(value)?.parent().map(Path::to_path_buf)
            }
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => match value.as_ref() {
                Expr::Attribute(ast::ExprAttribute { value: base, attr, .. }) if attr.as_str() == "parents" => {
                    let index = slice.as_number_literal_expr()?.value.as_int()?.as_usize()?;
                    self.static_path(base)?.ancestors().nth(index.checked_add(1)?).map(Path::to_path_buf)
                }
                _ => None,
            },
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                match (self.qualified(func)?.as_str(), &*arguments.args) {
                    ("pathlib.Path" | "os.path.join", args) => {
                        args.iter().try_fold(PathBuf::new(), |path, arg| Some(path.join(self.static_path(arg)?)))
                    }
                    ("os.path.dirname", [arg]) => self.static_path(arg)?.parent().map(Path::to_path_buf),
                    _ => None,
                }
            }
            _ => None,
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

// The module a `mock.patch` target f-string names before its first interpolated name, `pkg.report` for
// `f"pkg.report.{name}"`. A target that starts with the interpolation names no module the run can find.
fn computed_patch_module(target: &Expr) -> Option<&str> {
    let Expr::FString(fstring) = target else {
        return None;
    };
    let mut elements = fstring.value.elements();
    let Some(ast::InterpolatedStringElement::Literal(literal)) = elements.next() else {
        return None;
    };
    if !matches!(elements.next(), Some(ast::InterpolatedStringElement::Interpolation(_))) {
        return None;
    }
    literal.value.rsplit_once('.').map(|(module, _)| module)
}

// Whether a call takes the object an attribute belongs to first and the attribute's name second, as `setattr`,
// `monkeypatch.delattr` or `patch.object` does.
fn named_attribute_call(func: &Expr) -> bool {
    match last_segment(func) {
        Some("setattr" | "delattr") => true,
        Some("object") => {
            matches!(func, Expr::Attribute(ast::ExprAttribute { value, .. }) if last_segment(value) == Some("patch"))
        }
        _ => false,
    }
}

// Whether a call is `patch.object`, as `mock.patch.object` or `mocker.patch.object` is.
fn patch_object(func: &Expr) -> bool {
    matches!(func, Expr::Attribute(ast::ExprAttribute { attr, value, .. })
        if attr.as_str() == "object" && last_segment(value) == Some("patch"))
}

// The globals a `runpy` call returns: those of the module it runs, or of an unknown one no static path names.
#[derive(Clone, PartialEq)]
enum Globals {
    Module(String),
    Unknown,
}

// Whether an expression is an `__import__` call, or an attribute chain on one, whose attributes are submodules of the
// package the call returns.
fn is_dunder_import_chain(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_dunder_import_chain(value),
        Expr::Call(ast::ExprCall { func, .. }) => last_segment(func) == Some("__import__"),
        _ => false,
    }
}

// Whether an `__import__` call names a `fromlist` that makes it return the module it names rather than the top package:
// a list or tuple literal with items, or any expression that may hold some. A missing or empty `fromlist` gives the top
// package.
fn names_fromlist(arguments: &ast::Arguments) -> bool {
    match arguments.find_argument_value("fromlist", 3) {
        None | Some(Expr::NoneLiteral(_)) => false,
        Some(Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) => !elts.is_empty(),
        Some(_) => true,
    }
}

// The module a call loads by a string literal, as `importlib.import_module("pkg.mod")`, `__import__("pkg.mod")` or
// `pytest.importorskip("pkg.mod")` does, whose attributes are the names the call's result reads. Without a `fromlist`,
// `__import__` returns the top package of the name, not the module it names.
fn loaded_module(expr: &Expr) -> Option<&str> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    let name = last_segment(func)?;
    if !matches!(name, "import_module" | "__import__" | "importorskip") {
        return None;
    }
    let module = arguments.args.first().and_then(|module| module.as_string_literal_expr())?.value.to_str();
    match module.split_once('.') {
        Some((package, _)) if name == "__import__" && !names_fromlist(arguments) => Some(package),
        _ => Some(module),
    }
}

// The spec a `module_from_spec(spec)` call loads a module from.
fn module_from_spec_arg(expr: &Expr) -> Option<&Expr> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    if last_segment(func) != Some("module_from_spec") {
        return None;
    }
    arguments.args.first()
}

// The path with its `.` and `..` parts resolved without touching the file system, so that a path under the root has
// the root as its prefix.
fn lexically_normalized(path: &Path) -> PathBuf {
    let mut normal = PathBuf::new();
    for component in path.components().filter(|component| *component != Component::CurDir) {
        match component {
            Component::ParentDir if normal.file_name().is_some() => _ = normal.pop(),
            _ => normal.push(component),
        }
    }
    normal
}

// The dotted module a path under the search root names, as `pkg/mod.py` names `pkg.mod` and `pkg/__init__.py` names
// `pkg`.
fn module_of(relative: &Path) -> Option<String> {
    let mut parts = relative.iter().map(|part| part.to_str()).collect::<Option<Vec<&str>>>()?;
    let file = parts.pop()?.strip_suffix(".py")?;
    parts.push(file);
    let parts = module_parts(&parts);
    (!parts.is_empty() && parts.iter().all(|part| is_identifier(part))).then(|| parts.join("."))
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
        }
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
                let parameters = parameter_names(&function.parameters);
                let shadowed = self.shadow(parameters.clone());
                self.loaded_modules.enter(parameters);
                visitor::walk_stmt(self, stmt);
                self.loaded_modules.functions.pop();
                restore_bindings(&mut self.loop_modules, shadowed);
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
                        self.aliases.insert(asname.as_str(), full.to_owned())
                    } else {
                        let root = full.split('.').next().unwrap_or_default();
                        if top_level {
                            self.plain_modules.push((root.to_owned(), root.to_owned()));
                        }
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
                self.base_classes.extend(class.bases().iter().filter_map(|base| {
                    UnqualifiedName::from_expr(base)
                        .and_then(|name| name.segments().last().map(|last| (*last).to_owned()))
                }));
                visitor::walk_stmt(self, stmt);
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
                    self.edges.entry(module).or_default().insert(attr.as_str().to_owned());
                }
                // `lazy_fixture(db.__name__)` and `usefixtures(db.__name__)` request a fixture by its function's name.
                if attr.as_str() == "__name__"
                    && let Some(name) = value.as_name_expr()
                    && self.parts.has(Parts::FIXTURES)
                {
                    self.fixture_requests.insert(name.id.to_string());
                }
                if let Some(path) = self.qualified(expr) {
                    Self::record_path(&mut self.edges, &path);
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
                    Self::record_path(&mut self.references, text);
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
            self.edges.entry(module.to_owned()).or_default().insert(name.to_owned());
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
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            targets.iter().flat_map(|target| stored_names(target)).collect()
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) | Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => {
            stored_names(target)
        }
        Stmt::For(ast::StmtFor { target, .. }) => stored_names(target),
        Stmt::With(ast::StmtWith { items, .. }) => items
            .iter()
            .filter_map(|item| item.optional_vars.as_deref())
            .flat_map(|target| stored_names(target))
            .collect(),
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

fn stored_names(target: &Expr) -> Vec<&str> {
    match target {
        Expr::Name(name) => vec![name.id.as_str()],
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().flat_map(|elt| stored_names(elt)).collect()
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => stored_names(value),
        _ => Vec::new(),
    }
}

fn parameter_names(parameters: &ast::Parameters) -> Vec<&str> {
    parameters.iter().map(|parameter| parameter.name().as_str()).collect()
}

fn is_pytest_plugins(target: &Expr) -> bool {
    target.as_name_expr().is_some_and(|name| name.id.as_str() == "pytest_plugins")
}

impl FactVisitor<'_> {
    // The parameters of a function request fixtures, and those of a test or fixture wherever pytest collects them.
    fn request_parameters(&mut self, function: &ast::StmtFunctionDef) {
        let parameters: Vec<String> =
            function.parameters.iter().map(|parameter| parameter.name().to_string()).collect();
        if function.name.starts_with("test") || has_decorator(&function.decorator_list, &["fixture"]) {
            self.pytest_requests.extend(parameters.clone());
        }
        self.fixture_requests.extend(parameters);
    }

    // Plugins a test module loads may request fixtures of the run by name, which the index records as a request of the
    // name `pytest_plugins`, one no fixture can take. pytest's own plugins, such as `pytester`, request none of them.
    fn load_plugins(&mut self, value: &Expr) {
        let plugins: Vec<&Expr> = match value {
            Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().collect(),
            single => vec![single],
        };
        let builtin = |plugin: &&Expr| {
            plugin.as_string_literal_expr().is_some_and(|name| {
                let name = name.value.to_str();
                name == "pytester" || name.starts_with("_pytest")
            })
        };
        if !plugins.iter().all(builtin) {
            self.fixture_requests.insert("pytest_plugins".to_owned());
        }
    }
}

// The code of a docstring's `>>>` example lines, without the prompt.
pub(crate) fn doctest_code(text: &str) -> impl Iterator<Item = &str> {
    text.lines().filter_map(|line| line.trim_start().strip_prefix(">>>"))
}

// The identifiers of the `>>>` example lines of a docstring.
fn doctest_names(text: &str) -> impl Iterator<Item = String> + '_ {
    doctest_code(text)
        .flat_map(|code| code.split(|char: char| !(char.is_alphanumeric() || char == '_')))
        .filter(|word| is_identifier(word))
        .map(str::to_owned)
}

// The level, module and names a `from module import name, other as alias` example imports.
pub(crate) fn from_import(code: &str) -> Option<(u32, &str, Vec<&str>)> {
    let (module, names) = code.trim_start().strip_prefix("from ")?.split_once(" import ")?;
    let module = module.trim();
    let name = module.trim_start_matches('.');
    let level = u32::try_from(module.len() - name.len()).ok()?;
    let names = names
        .trim()
        .trim_start_matches('(')
        .trim_end_matches(')')
        .split(',')
        .filter_map(|name| name.split_whitespace().next())
        .filter(|name| *name != "*" && is_identifier(name))
        .collect();
    Some((level, name, names))
}

// The argument that names the classes a call checks or splits: the second of `isinstance(error, Cls)`, and the first
// of `group.split(Cls)` or `group.subgroup(Cls)` on an exception group.
fn class_argument<'a>(func: &Expr, arguments: &'a ast::Arguments) -> Option<&'a Expr> {
    if func.as_name_expr().is_some_and(|name| name.id.as_str() == "isinstance") {
        arguments.args.get(1)
    } else if func.as_attribute_expr().is_some_and(|attribute| matches!(attribute.attr.as_str(), "split" | "subgroup"))
    {
        arguments.args.first()
    } else {
        None
    }
}

// The dotted name `segments` stand for through the import that binds their head, such as `pkg.models.Partition` for
// `models.Partition` after `from pkg import models`.
pub(crate) fn qualified(segments: &[&str], aliases: &FxHashMap<&str, String>) -> Option<String> {
    let (first, rest) = segments.split_first()?;
    let module = aliases.get(first)?;
    Some(std::iter::once(module.as_str()).chain(rest.iter().copied()).collect::<Vec<_>>().join("."))
}

impl FactVisitor<'_> {
    fn qualified(&self, expr: &Expr) -> Option<String> {
        qualified(UnqualifiedName::from_expr(expr)?.segments(), &self.aliases)
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

    // Records the classes an `except` clause names, each alternative of a tuple on its own.
    fn catch(&mut self, expr: &Expr) {
        if !self.parts.has(Parts::CAUGHT) {
            return;
        }
        if let Expr::Tuple(ast::ExprTuple { elts, .. }) = expr {
            for elt in elts {
                self.catch(elt);
            }
        } else if let Some(path) = self.class_path(expr) {
            self.caught.push(path);
        }
    }

    // The module and the names a call sets through a literal or a loop over string literals, as `setattr(module, "NAME")`
    // or `patch.object(module, name)` does.
    fn named_names(&self, arguments: &ast::Arguments) -> Vec<(String, Vec<String>)> {
        match &*arguments.args {
            [target, Expr::StringLiteral(name), ..] => self
                .receivers(target)
                .into_iter()
                .map(|module| (module, vec![name.value.to_str().to_owned()]))
                .collect(),
            [target, Expr::Name(name), ..] => {
                let Some(strings) = self.loop_strings.get(name.id.as_str()) else {
                    return Vec::new();
                };
                self.receivers(target).into_iter().map(|module| (module, strings.clone())).collect()
            }
            _ => Vec::new(),
        }
    }

    // The modules a call's receiver stands for: the module it imports, or each module a loop over a literal of modules
    // binds it to.
    fn receivers(&self, target: &Expr) -> Vec<String> {
        match target.as_name_expr().and_then(|name| self.loop_modules.get(name.id.as_str())) {
            Some(modules) => modules.clone(),
            None => self.qualified(target).into_iter().collect(),
        }
    }

    // Records the names other code reaches on a module by a store or a call that names them, which a rename would break.
    fn record_references(&mut self, module: String, names: impl IntoIterator<Item = String>) {
        self.references.entry(module).or_default().extend(names);
    }

    // The module and the names a call writes on it: a `<receiver>.setattr` call, a `patch.object` call, or a `mock.patch`
    // of a dotted string, which pytest and `unittest.mock` split at the last dot. The receiver's name does not matter, so
    // any object counts.
    fn module_writes(
        func: &Expr,
        arguments: &ast::Arguments,
        named: Vec<(String, Vec<String>)>,
    ) -> Vec<(String, Vec<String>)> {
        let setattr = matches!(func, Expr::Attribute(ast::ExprAttribute { attr, .. }) if attr.as_str() == "setattr");
        match &*arguments.args {
            [Expr::StringLiteral(path), ..] if setattr || last_segment(func) == Some("patch") => {
                let Some((module, name)) = path.value.to_str().rsplit_once('.') else {
                    return Vec::new();
                };
                vec![(module.to_owned(), vec![name.to_owned()])]
            }
            _ if setattr || patch_object(func) => named,
            _ => Vec::new(),
        }
    }

    // Records the module a write under a name the run cannot know reaches: `setattr(module, name, value)` with a name
    // that is neither a literal nor a loop over string literals, or a `mock.patch` target f-string spelling the module.
    fn computed_write(&mut self, func: &Expr, arguments: &ast::Arguments) {
        let module = match &*arguments.args {
            [target, name, ..]
                if named_attribute_call(func)
                    && !name.is_string_literal_expr()
                    && !name.as_name_expr().is_some_and(|name| self.loop_strings.contains_key(name.id.as_str())) =>
            {
                self.qualified(target)
            }
            [target, ..] if last_segment(func) == Some("patch") => computed_patch_module(target).map(str::to_owned),
            _ => None,
        };
        self.computed_writes.extend(module);
    }

    // The imports and dotted names a docstring's examples make, which the run reads as it reads the code's own, so a
    // doctest's `from module import name` keeps `name` used.
    fn doctests(&mut self, text: &str) {
        for code in doctest_code(text) {
            if let Some((level, module, names)) = from_import(code)
                && let Some(resolved) =
                    resolve_imported_module_path(level, (!module.is_empty()).then_some(module), self.module_path)
            {
                self.edges.entry(resolved.into_owned()).or_default().extend(names.into_iter().map(str::to_owned));
            }
            // A doctest reads a name off a module, which no patch of the module is, but the call-site rules must see it.
            for path in code.split(|char: char| !(char.is_alphanumeric() || matches!(char, '_' | '.'))) {
                if path.contains('.') && path.split('.').all(is_identifier) {
                    Self::record_path(&mut self.doctest_references, path);
                }
            }
        }
    }

    fn record_path(names: &mut BTreeMap<String, BTreeSet<String>>, path: &str) {
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
