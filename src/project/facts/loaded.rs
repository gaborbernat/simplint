use std::path::{Component, Path, PathBuf};

use ruff_python_ast::{self as ast, Expr};
use ruff_python_stdlib::identifiers::is_identifier;

use crate::project::receivers::module_parts;
use crate::syntax::last_segment;

use super::Channel;
use super::visitor::FactVisitor;

impl<'a> FactVisitor<'a> {
    // Binds each name a plain assignment sets to the modules its value loads, in the scope the assignment is in. A spec a
    // `spec_from_file_location` call builds is bound the same way, to the module its path names.
    pub(super) fn bind_loaded_module(&mut self, targets: &'a [Expr], value: &'a Expr) {
        let modules = self.loaded_modules_of(value);
        let spec = self.spec_module(value);
        let unknown = modules.is_empty() && self.is_unknown_module(value);
        for name in targets.iter().filter_map(Expr::as_name_expr) {
            let bound = name.id.as_str();
            self.specs.insert(bound, spec.clone());
            if unknown {
                self.unknown_modules.insert(bound.to_owned());
            }
            self.scope_mut().bind(bound, &modules);
        }
    }

    // The project modules an expression loads or names: a string-named import call, a module built from a spec, a
    // variable bound to modules, or a submodule read off the package an `__import__` call returns.
    pub(super) fn loaded_modules_of(&self, expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Name(name) => self.loaded_named(name.id.as_str()),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if is_dunder_import_chain(value) => {
                self.loaded_modules_of(value).into_iter().map(|module| format!("{module}.{attr}")).collect()
            }
            _ => loaded_module(expr).map(str::to_owned).or_else(|| self.module_from_spec(expr)).into_iter().collect(),
        }
    }

    // Binds each name a plain assignment sets to the globals its value returns, or unbinds it to unknown when another
    // value sets it.
    pub(super) fn bind_globals(&mut self, targets: &'a [Expr], value: &'a Expr) {
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

    // Records a string key read off globals, which imports the name from the module the globals run, or may read any
    // module's name when no static path names it.
    pub(super) fn read_globals(&mut self, value: &Expr, key: &str) {
        match self.globals_of(value) {
            Some(Globals::Module(module)) => {
                _ = self.names[Channel::Import].entry(module).or_default().insert(key.to_owned());
            }
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

// The globals a `runpy` call returns: those of the module it runs, or of an unknown one no static path names.
#[derive(Clone, PartialEq)]
pub(super) enum Globals {
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
pub(super) fn lexically_normalized(path: &Path) -> PathBuf {
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
