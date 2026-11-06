use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet};

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
    // The module-level names bound once to a false constant, as `_false = 0`, whose `if` bodies never run.
    pub(super) falsy_constants: FxHashSet<&'a str>,
    pub(super) runtime_sites: Vec<ImportSite>,
    pub(super) star_sites: Vec<ImportSite>,
    // The facts only some rules read that the run collects.
    pub(super) parts: Parts,
}

impl<'a> FactVisitor<'a> {
    pub(super) fn new(
        module_path: Option<&'a [String]>,
        own_module: String,
        parts: Parts,
        falsy_constants: FxHashSet<&'a str>,
    ) -> Self {
        Self {
            module_path,
            aliases: FxHashMap::default(),
            edges: BTreeMap::new(),
            references: BTreeMap::new(),
            assigned_attributes: Vec::new(),
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

impl<'a> Visitor<'a> for FactVisitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
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
            Expr::Attribute(ast::ExprAttribute { attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.assigned_attributes.push(attr.as_str().to_owned());
            }
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
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
