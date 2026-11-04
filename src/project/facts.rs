use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use ruff_python_ast::helpers::{ReturnStatementVisitor, resolve_imported_module_path};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_parser::parse_module;
use ruff_python_stdlib::identifiers::is_identifier;

use super::constructors::Constructors;
use super::receivers::{AttributeWrites, module_name};
use crate::rules::exports::{Exports, dunder_all};
use crate::rules::module_statements;

// The names a file uses from other modules, before they resolve to files; parsing finds them and the cache keeps them.
// Names are grouped by module, so each module resolves once per file.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileFacts {
    pub(super) imports: Vec<ModuleNames>,
    pub(super) references: Vec<ModuleNames>,
    pub(super) declared: Vec<String>,
    pub(super) assigned_attributes: Vec<String>,
    // The attributes read off module objects whose module parsing cannot tell, any of which may be a name of this run.
    pub(super) module_reads: Vec<String>,
    pub(super) base_classes: Vec<String>,
    // The modules the file star-imports, which hand it whatever names their `import *` exports.
    pub(super) star_imports: Vec<String>,
    // The names a static `__all__` lists, which are all `import *` takes; without one it takes every public name.
    pub(super) exports: Option<Vec<String>>,
    // Whether `__all__` is built in a way parsing cannot follow, or a module-level `__getattr__` supplies names lazily,
    // so the module's exports are unknown.
    pub(super) exports_unknown: bool,
    // The names the module's own top-level statements import from other modules, which `__all__`, a redundant alias
    // such as `from .core import run as run`, or the absence of `__all__` may re-export.
    pub(super) bindings: Vec<ImportBinding>,
    // The names the module's top-level statements define, which a module without `__all__` exports.
    pub(super) defined: Vec<String>,
    pub(super) attribute_writes: AttributeWrites,
    pub(super) constructors: Constructors,
    // Whether the file reads a docstring off an object, through `__doc__` or a `getdoc` call.
    pub(super) reads_docstrings: bool,
}

impl FileFacts {
    // The module path, such as `["package", "module"]`, is what relative imports resolve against. A file outside any
    // package has none, and resolves them against `importable`, the module path under its search root, if it has one.
    pub fn collect(path: &Path, module_path: Option<&[String]>, importable: Option<&[String]>) -> Option<Self> {
        let source = std::fs::read_to_string(path).ok()?;
        let parsed = parse_module(&source).ok()?;
        let mut imports = Imports {
            module_path: module_path.or(importable),
            aliases: HashMap::new(),
            edges: BTreeMap::new(),
            references: BTreeMap::new(),
            assigned_attributes: Vec::new(),
            base_classes: Vec::new(),
            stars: BTreeSet::new(),
            reads_docstrings: false,
        };
        imports.visit_body(&parsed.syntax().body);
        // A stub publishes what it declares, so its names are the API of the module it describes.
        let declared = if path.extension().is_some_and(|extension| extension == "pyi") {
            parsed.syntax().body.iter().flat_map(declared_names).collect()
        } else {
            Vec::new()
        };
        let attribute_writes =
            AttributeWrites::collect(&parsed.syntax().body, &imports.aliases, &module_name(module_path, path));
        let constructors =
            Constructors::collect(&source, &parsed.syntax().body, &imports.aliases, module_path.or(importable));
        Some(Self {
            imports: grouped(imports.edges),
            references: grouped(imports.references),
            declared,
            assigned_attributes: imports.assigned_attributes,
            module_reads: module_reads(&parsed.syntax().body),
            base_classes: imports.base_classes,
            star_imports: imports.stars.into_iter().collect(),
            exports: match dunder_all(&parsed.syntax().body) {
                Exports::Listed(names) => Some(names.into_iter().map(str::to_owned).collect()),
                Exports::Absent | Exports::Dynamic => None,
            },
            exports_unknown: matches!(dunder_all(&parsed.syntax().body), Exports::Dynamic)
                || has_module_getattr(&parsed.syntax().body),
            bindings: top_level_bindings(&parsed.syntax().body, module_path.or(importable)),
            defined: module_statements(&parsed.syntax().body).flat_map(declared_names).collect(),
            attribute_writes,
            constructors,
            reads_docstrings: imports.reads_docstrings,
        })
    }
}

// A module and the names used from it.
pub(super) type ModuleNames = (String, Vec<String>);

// A name a `from module import name as bound` statement binds; `redundant` marks `name as name`, which the typing spec
// reads as a re-export.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct ImportBinding {
    pub(super) module: String,
    pub(super) name: String,
    pub(super) bound: String,
    pub(super) redundant: bool,
}

fn top_level_bindings(body: &[Stmt], module_path: Option<&[String]>) -> Vec<ImportBinding> {
    module_statements(body)
        .filter_map(Stmt::as_import_from_stmt)
        .filter_map(|import| {
            let module = import.module.as_ref().map(ast::Identifier::as_str);
            let resolved = resolve_imported_module_path(import.level, module, module_path)?.into_owned();
            Some(import.names.iter().filter(|alias| alias.name.as_str() != "*").map(move |alias| ImportBinding {
                module: resolved.clone(),
                name: alias.name.to_string(),
                bound: alias.asname.as_ref().unwrap_or(&alias.name).to_string(),
                redundant: alias.asname.as_ref().is_some_and(|asname| asname.as_str() == alias.name.as_str()),
            }))
        })
        .flatten()
        .collect()
}

// PEP 562's module `__getattr__` answers for names the module does not bind, as lazily loaded submodules.
fn has_module_getattr(body: &[Stmt]) -> bool {
    module_statements(body)
        .any(|stmt| stmt.as_function_def_stmt().is_some_and(|def| def.name.as_str() == "__getattr__"))
}

fn grouped(names: BTreeMap<String, BTreeSet<String>>) -> Vec<ModuleNames> {
    names.into_iter().map(|(module, names)| (module, names.into_iter().collect())).collect()
}

struct Imports<'a> {
    module_path: Option<&'a [String]>,
    aliases: HashMap<&'a str, String>,
    edges: BTreeMap<String, BTreeSet<String>>,
    references: BTreeMap<String, BTreeSet<String>>,
    assigned_attributes: Vec<String>,
    base_classes: Vec<String>,
    stars: BTreeSet<String>,
    reads_docstrings: bool,
}

impl<'a> Visitor<'a> for Imports<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ImportFrom(ast::StmtImportFrom { module, names, level, .. }) => {
                let module = module.as_ref().map(ast::Identifier::as_str);
                if let Some(resolved) =
                    resolve_imported_module_path(*level, module, self.module_path).map(Cow::into_owned)
                {
                    if names.iter().any(|alias| alias.name.as_str() == "*") {
                        self.stars.insert(resolved.clone());
                    }
                    for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                        self.edges.entry(resolved.clone()).or_default().insert(alias.name.as_str().to_owned());
                        let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                        self.aliases.insert(bound, format!("{resolved}.{}", alias.name.as_str()));
                    }
                }
            }
            Stmt::Import(ast::StmtImport { names, .. }) => {
                for alias in names {
                    let full = alias.name.as_str();
                    if let Some(asname) = &alias.asname {
                        self.aliases.insert(asname.as_str(), full.to_owned())
                    } else {
                        let root = full.split('.').next().unwrap_or_default();
                        self.aliases.insert(root, root.to_owned())
                    };
                }
            }
            Stmt::ClassDef(class) => {
                self.base_classes.extend(class.bases().iter().filter_map(|base| {
                    UnqualifiedName::from_expr(base)
                        .and_then(|name| name.segments().last().map(|last| (*last).to_owned()))
                }));
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.reads_docstrings |= reads_docstring(expr);
        match expr {
            Expr::Attribute(ast::ExprAttribute { attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.assigned_attributes.push(attr.as_str().to_owned());
            }
            Expr::Attribute(_) => {
                if let Some(path) = self.qualified(expr) {
                    Self::record_path(&mut self.edges, &path);
                    return;
                }
            }
            Expr::StringLiteral(literal) => {
                let text = literal.value.to_str();
                if text.contains('.') && text.split('.').all(is_identifier) {
                    Self::record_path(&mut self.references, text);
                }
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
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

impl Imports<'_> {
    fn qualified(&self, expr: &Expr) -> Option<String> {
        let name = UnqualifiedName::from_expr(expr)?;
        let (first, rest) = name.segments().split_first()?;
        let module = self.aliases.get(first)?;
        Some(std::iter::once(module.as_str()).chain(rest.iter().copied()).collect::<Vec<_>>().join("."))
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
        Expr::Call(ast::ExprCall { func, arguments, .. }) => {
            let is_doc = |arg: &Expr| arg.as_string_literal_expr().is_some_and(|text| text.value.to_str() == "__doc__");
            UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().last() == Some(&"getdoc"))
                || func.as_name_expr().is_some_and(|name| name.id.as_str() == "getattr")
                    && arguments.args.get(1).is_some_and(is_doc)
        }
        _ => false,
    }
}

// A module loaded by a computed name, as `importlib.import_module(f"pkg.{name}")` or a test fixture typed `ModuleType`,
// hands over whichever module that name picks, so each attribute read off it may be a name of any module.
fn module_reads(body: &[Stmt]) -> Vec<String> {
    let mut modules = ModuleObjects::default();
    modules.visit_body(body);
    if modules.names.is_empty() {
        return Vec::new();
    }
    let mut reads = ModuleReads { modules: &modules.names, reads: BTreeSet::new() };
    reads.visit_body(body);
    reads.reads.into_iter().collect()
}

// The names the file binds to module objects, wherever it binds them; a name bound to a module anywhere counts
// everywhere, which can only keep more names.
#[derive(Default)]
struct ModuleObjects<'a> {
    names: HashSet<&'a str>,
}

impl<'a> Visitor<'a> for ModuleObjects<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let (targets, value, annotation) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => (targets.as_slice(), Some(value.as_ref()), None),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value, annotation, .. }) => {
                (std::slice::from_ref(target.as_ref()), value.as_deref(), Some(annotation.as_ref()))
            }
            _ => (&[][..], None, None),
        };
        if value.is_some_and(|value| self.is_module(value)) || annotation.is_some_and(is_module_type) {
            self.names.extend(targets.iter().filter_map(Expr::as_name_expr).map(|name| name.id.as_str()));
        }
        visitor::walk_stmt(self, stmt);
        // A pytest fixture that returns a module hands it to each test under the fixture's name.
        if let Stmt::FunctionDef(function) = stmt {
            let mut returns = ReturnStatementVisitor::default();
            returns.visit_body(&function.body);
            let returns_module =
                returns.returns.iter().filter_map(|stmt| stmt.value.as_deref()).any(|value| self.is_module(value));
            if returns_module {
                self.names.insert(function.name.as_str());
                self.names.extend(
                    function
                        .decorator_list
                        .iter()
                        .filter_map(|decorator| decorator.expression.as_call_expr()?.arguments.find_keyword("name"))
                        .filter_map(|keyword| Some(keyword.value.as_string_literal_expr()?.value.to_str())),
                );
            }
        }
    }

    fn visit_parameter(&mut self, parameter: &'a ast::Parameter) {
        if parameter.annotation.as_deref().is_some_and(is_module_type) {
            self.names.insert(parameter.name.as_str());
        }
        visitor::walk_parameter(self, parameter);
    }
}

impl ModuleObjects<'_> {
    // A module load, or a name already bound to a module, as is or through `typing.cast`.
    fn is_module(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => self.names.contains(name.id.as_str()),
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().last() == Some(&"cast")) =>
            {
                arguments.args.get(1).is_some_and(|value| self.is_module(value))
            }
            _ => is_module_load(expr),
        }
    }
}

fn is_module_load(expr: &Expr) -> bool {
    match expr {
        Expr::Call(ast::ExprCall { func, .. }) => match func.as_ref() {
            Expr::Name(name) => matches!(name.id.as_str(), "import_module" | "__import__"),
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => attr == "import_module",
            _ => false,
        },
        Expr::Subscript(ast::ExprSubscript { value, .. }) => {
            UnqualifiedName::from_expr(value).is_some_and(|name| name.segments() == ["sys", "modules"])
        }
        _ => false,
    }
}

fn is_module_type(annotation: &Expr) -> bool {
    UnqualifiedName::from_expr(annotation)
        .is_some_and(|name| matches!(name.segments(), ["ModuleType"] | ["types", "ModuleType"]))
}

struct ModuleReads<'a, 'm> {
    modules: &'m HashSet<&'a str>,
    reads: BTreeSet<String>,
}

impl<'a> Visitor<'a> for ModuleReads<'a, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let read = match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                value.as_name_expr().filter(|name| self.modules.contains(name.id.as_str())).map(|_| attr.as_str())
            }
            Expr::Call(ast::ExprCall { func, arguments, .. })
                if func.as_name_expr().is_some_and(|name| matches!(name.id.as_str(), "getattr" | "hasattr")) =>
            {
                match &*arguments.args {
                    [Expr::Name(module), Expr::StringLiteral(attribute), ..]
                        if self.modules.contains(module.id.as_str()) =>
                    {
                        Some(attribute.value.to_str())
                    }
                    _ => None,
                }
            }
            _ => None,
        };
        self.reads.extend(read.map(str::to_owned));
        visitor::walk_expr(self, expr);
    }
}

fn declared_names(stmt: &Stmt) -> Vec<String> {
    match stmt {
        Stmt::FunctionDef(function) => vec![function.name.to_string()],
        Stmt::ClassDef(class) => vec![class.name.to_string()],
        Stmt::Assign(ast::StmtAssign { targets, .. }) => {
            targets.iter().filter_map(Expr::as_name_expr).map(|name| name.id.to_string()).collect()
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. }) => {
            target.as_name_expr().map(|name| name.id.to_string()).into_iter().collect()
        }
        _ => Vec::new(),
    }
}
