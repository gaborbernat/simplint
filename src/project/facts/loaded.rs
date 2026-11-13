use std::path::{Component, Path, PathBuf};

use ruff_python_ast::{self as ast, Expr};
use ruff_python_stdlib::identifiers::is_identifier;

use crate::project::receivers::module_parts;
use crate::syntax::last_segment;

use super::Channel;
use super::visitor::FactVisitor;

impl<'a> FactVisitor<'a> {
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

    pub(super) fn loaded_modules_of(&self, expr: &Expr) -> Vec<String> {
        match expr {
            Expr::Name(name) => self.loaded_named(name.id.as_str()),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if is_dunder_import_chain(value) => {
                self.loaded_modules_of(value).into_iter().map(|module| format!("{module}.{attr}")).collect()
            }
            _ => loaded_module(expr)
                .map(str::to_owned)
                .or_else(|| self.spec_module(module_from_spec_arg(expr)?))
                .into_iter()
                .collect(),
        }
    }

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

    fn file_module(&self, expr: &Expr) -> Option<String> {
        let root = self.root.as_deref()?;
        let path = lexically_normalized(&root.join(self.static_path(expr)?));
        module_of(path.strip_prefix(root).ok()?)
    }

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

    fn is_unknown_module(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Name(name) => self.unknown_modules.contains(name.id.as_str()),
            _ => module_from_spec_arg(expr).is_some(),
        }
    }

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

    pub(super) fn read_globals(&mut self, value: &Expr, key: &str) {
        match self.globals_of(value) {
            Some(Globals::Module(module)) => {
                _ = self.names[Channel::Import].entry(module).or_default().insert(key.to_owned());
            }
            Some(Globals::Unknown) => _ = self.unresolved_globals.insert(key.to_owned()),
            None => {}
        }
    }
}

#[derive(Clone, PartialEq)]
pub(super) enum Globals {
    Module(String),
    Unknown,
}

fn is_dunder_import_chain(expr: &Expr) -> bool {
    match expr {
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_dunder_import_chain(value),
        Expr::Call(ast::ExprCall { func, .. }) => last_segment(func) == Some("__import__"),
        _ => false,
    }
}

// Without a `fromlist`, `__import__` returns the top package of the name, not the module it names.
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

fn names_fromlist(arguments: &ast::Arguments) -> bool {
    match arguments.find_argument_value("fromlist", 3) {
        None | Some(Expr::NoneLiteral(_)) => false,
        Some(Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) => !elts.is_empty(),
        Some(_) => true,
    }
}

fn module_from_spec_arg(expr: &Expr) -> Option<&Expr> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    if last_segment(func) != Some("module_from_spec") {
        return None;
    }
    arguments.args.first()
}

// Resolves without touching the file system, so a path under the root keeps the root as its prefix.
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

fn module_of(relative: &Path) -> Option<String> {
    let mut parts = relative.iter().map(|part| part.to_str()).collect::<Option<Vec<&str>>>()?;
    let file = parts.pop()?.strip_suffix(".py")?;
    parts.push(file);
    let parts = module_parts(&parts);
    (!parts.is_empty() && parts.iter().all(|part| is_identifier(part))).then(|| parts.join("."))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use test_case::test_case;

    use super::super::{FileFacts, Parts};
    use super::module_of;

    // Without a package around the file there is no root to name the module a path holds, so the key reads as one of
    // any module.
    #[test_case(None, &["value"] ; "outside_any_package")]
    #[test_case(Some(&["pkg", "main"]), &[] ; "inside_a_package")]
    fn run_path_names_its_module_under_the_package_root(module_path: Option<&[&str]>, reads: &[&str]) {
        let module_path: Option<Vec<String>> =
            module_path.map(|parts| parts.iter().map(|&part| part.to_owned()).collect());
        let source = "import runpy\n\nprint(runpy.run_path(\"helper.py\")[\"value\"])\n";
        let path = Path::new("/project/pkg/main.py");
        let facts = FileFacts::collect(path, source, module_path.as_deref(), None, Parts::default()).expect("facts");
        assert_eq!(facts.module_reads, reads);
    }

    #[test_case("pkg/mod.py", Some("pkg.mod") ; "module")]
    #[test_case("pkg/__init__.py", Some("pkg") ; "package")]
    #[test_case("", None ; "the_root_itself")]
    #[test_case("pkg/mod.txt", None ; "not_python")]
    fn module_of_names_the_module_a_path_holds(relative: &str, expected: Option<&str>) {
        assert_eq!(module_of(Path::new(relative)).as_deref(), expected);
    }

    #[cfg(unix)]
    #[test]
    fn module_of_a_path_that_is_not_utf8_is_unknown() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;

        assert_eq!(module_of(Path::new(OsStr::from_bytes(b"p\xffkg/mod.py"))), None);
    }
}
