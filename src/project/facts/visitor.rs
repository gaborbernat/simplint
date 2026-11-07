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
        .filter(|import| import.top_level)
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

pub(super) struct FactVisitor<'a> {
    pub(super) module_path: Option<&'a [String]>,
    pub(super) aliases: FxHashMap<&'a str, String>,
    pub(super) edges: BTreeMap<String, BTreeSet<String>>,
    pub(super) references: BTreeMap<String, BTreeSet<String>>,
    pub(super) assigned_attributes: Vec<String>,
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
    // The names bound to a module a string-named import call loads, mapped to that module, so `mod.name` reads a name of
    // it. A name bound to anything else too maps to `None`.
    pub(super) loaded_modules: FxHashMap<&'a str, Option<String>>,
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
    string_rows: FxHashMap<&'a str, Option<Vec<Vec<String>>>>,
    // The names a `for` over string literals binds, with the strings each takes in turn.
    loop_strings: FxHashMap<&'a str, Vec<String>>,
    // The file being visited and the directory its module path is relative to, both normalized, which the paths a
    // spec loads resolve against.
    file: PathBuf,
    root: Option<PathBuf>,
    // The module-level names bound once to a false constant, as `_false = 0`, whose `if` bodies never run.
    pub(super) falsy_constants: FxHashSet<&'a str>,
    pub(super) runtime_sites: Vec<ImportSite>,
    pub(super) star_sites: Vec<ImportSite>,
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
        let file = normalized(file);
        let root = module_path.and_then(|module_path| file.ancestors().nth(module_path.len())).map(normalized);
        Self {
            module_path,
            aliases: FxHashMap::default(),
            edges: BTreeMap::new(),
            references: BTreeMap::new(),
            assigned_attributes: Vec::new(),
            computed_writes: BTreeSet::new(),
            base_classes: Vec::new(),
            stars: BTreeSet::new(),
            from_statements: Vec::new(),
            top_level: true,
            reads_docstrings: false,
            reads_class_names: false,
            reflected_names: BTreeSet::new(),
            module_aliases: Vec::new(),
            own_module,
            caught: Vec::new(),
            fixture_requests: BTreeSet::new(),
            pytest_requests: BTreeSet::new(),
            fixture_names: FxHashSet::default(),
            doctest_names: BTreeSet::new(),
            handler_depth: 0,
            runtime: Runs::Always,
            falsy_constants,
            loaded_modules: FxHashMap::default(),
            specs: FxHashMap::default(),
            unknown_modules: FxHashSet::default(),
            globals: FxHashMap::default(),
            unresolved_globals: BTreeSet::new(),
            string_rows: FxHashMap::default(),
            loop_strings: FxHashMap::default(),
            file,
            root,
            parts,
            runtime_sites: Vec::new(),
            star_sites: Vec::new(),
        }
    }
}

impl<'a> FactVisitor<'a> {
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
                if self.parts.has(Parts::STAR_SITES) {
                    self.star_sites.push(ImportSite::new(resolved.clone(), Vec::new(), stmt.range(), runtime));
                }
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
    // Binds each name a plain assignment sets to the module its value loads, or unbinds it when another value sets it.
    // A spec a `spec_from_file_location` call builds is bound the same way, to the module its path names.
    fn bind_loaded_module(&mut self, targets: &'a [Expr], value: &'a Expr) {
        let module = self.loaded_module_of(value);
        let spec = self.spec_module(value);
        let unknown = module.is_none() && self.is_unknown_module(value);
        for name in targets.iter().filter_map(Expr::as_name_expr) {
            let bound = name.id.as_str();
            self.specs.insert(bound, spec.clone());
            if unknown {
                self.unknown_modules.insert(bound.to_owned());
            }
            let slot = self.loaded_modules.entry(bound).or_insert_with(|| module.clone());
            if *slot != module {
                *slot = None;
            }
        }
    }

    // The project module an expression loads or names: a string-named import call, a module built from a spec, or a
    // variable bound to one.
    fn loaded_module_of(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => self.loaded_modules.get(name.id.as_str()).cloned().flatten(),
            _ => loaded_module(expr).map(str::to_owned).or_else(|| self.module_from_spec(expr)),
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
            let slot = self.string_rows.entry(name.id.as_str()).or_insert_with(|| rows.clone());
            if *slot != rows {
                *slot = None;
            }
        }
    }

    // The rows of a tuple, list or set of string literals, or of the rows a name bound to one holds.
    fn string_rows(&self, expr: &Expr) -> Option<Vec<Vec<String>>> {
        match expr {
            Expr::Name(name) => self.string_rows.get(name.id.as_str()).cloned().flatten(),
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
        let path = normalized(&root.join(self.static_path(expr)?));
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

// The globals a `runpy` call returns: those of the module it runs, or of an unknown one no static path names.
#[derive(Clone, PartialEq)]
enum Globals {
    Module(String),
    Unknown,
}

// The module a call loads by a string literal, as `importlib.import_module("pkg.mod")`, `__import__("pkg.mod")` or
// `pytest.importorskip("pkg.mod")` does, whose attributes are the names the call's result reads.
fn loaded_module(expr: &Expr) -> Option<&str> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    if !matches!(last_segment(func), Some("import_module" | "__import__" | "importorskip")) {
        return None;
    }
    arguments.args.first().and_then(|module| module.as_string_literal_expr()).map(|module| module.value.to_str())
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
fn normalized(path: &Path) -> PathBuf {
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
    if file != "__init__" {
        parts.push(file);
    }
    (!parts.is_empty() && parts.iter().all(|part| is_identifier(part))).then(|| parts.join("."))
}

impl<'a> Visitor<'a> for FactVisitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
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
                visitor::walk_stmt(self, stmt);
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
            Stmt::For(ast::StmtFor { target, iter, .. }) => {
                let bound = self.loop_bindings(target, iter);
                let previous: Vec<(&'a str, Option<Vec<String>>)> =
                    bound.iter().map(|(name, _)| (*name, self.loop_strings.get(name).cloned())).collect();
                self.loop_strings.extend(bound);
                visitor::walk_stmt(self, stmt);
                for (name, strings) in previous {
                    match strings {
                        Some(strings) => _ = self.loop_strings.insert(name, strings),
                        None => _ = self.loop_strings.remove(name),
                    }
                }
            }
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
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.assigned_attributes.push(attr.as_str().to_owned());
                // A store on a module object reaches the module's name as `setattr` does, so the module's own name
                // stays a module attribute and is not localized or renamed as one nothing reads.
                if let Some(module) = self.qualified(value) {
                    self.references.entry(module).or_default().insert(attr.as_str().to_owned());
                }
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                // A read off a loaded module is an import of the name, as `module.name` after `import module` is.
                if let Some(module) = self.loaded_module_of(value) {
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
                // The name `@pytest.fixture(name="db")` gives is the fixture's own, not a request of it.
                if self.parts.has(Parts::FIXTURES)
                    && last_segment(func) == Some("fixture")
                    && let Some(name) = arguments.find_keyword("name")
                {
                    self.fixture_names.insert(name.value.range());
                }
                if let [target, Expr::StringLiteral(attribute), ..] = &*arguments.args {
                    let attribute = attribute.value.to_str();
                    if func.as_name_expr().is_some_and(|name| name.id.as_str() == "setattr") {
                        self.assigned_attributes.push(attribute.to_owned());
                    }
                    if let Some(module) = self.qualified(target) {
                        self.references.entry(module).or_default().insert(attribute.to_owned());
                    }
                }
                // A name a loop over string literals binds names each of their attributes, as `setattr(module, name)`
                // does for each string.
                if let [target, Expr::Name(name), ..] = &*arguments.args
                    && named_attribute_call(func)
                    && let Some(names) = self.loop_strings.get(name.id.as_str())
                    && let Some(module) = self.qualified(target)
                {
                    self.references.entry(module).or_default().extend(names.iter().cloned());
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
    }
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

// The identifiers of the `>>>` example lines of a docstring.
fn doctest_names(text: &str) -> impl Iterator<Item = String> + '_ {
    text.lines()
        .filter(|line| line.trim_start().starts_with(">>>"))
        .flat_map(|line| line.split(|char: char| !(char.is_alphanumeric() || char == '_')))
        .filter(|word| is_identifier(word))
        .map(str::to_owned)
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
