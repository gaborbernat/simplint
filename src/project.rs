use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use rayon::prelude::*;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_parser::parse_module;

pub struct Project {
    imported: HashMap<String, HashSet<String>>,
    assigned_attributes: HashSet<String>,
    base_classes: HashSet<String>,
}

impl Project {
    pub fn index(files: &[PathBuf], entry_points: &[String]) -> Self {
        let modules: Vec<ModuleFacts> = files
            .par_iter()
            .filter_map(|path| {
                let source = std::fs::read_to_string(path).ok()?;
                let parsed = parse_module(&source).ok()?;
                let mut imports = Imports {
                    package: package_parts(path),
                    aliases: HashMap::new(),
                    edges: Vec::new(),
                    assigned_attributes: Vec::new(),
                    base_classes: Vec::new(),
                };
                imports.visit_body(&parsed.syntax().body);
                let own = module_name(path);
                imports.edges.retain(|(module, _)| *module != own);
                Some(ModuleFacts {
                    edges: imports.edges,
                    assigned_attributes: imports.assigned_attributes,
                    base_classes: imports.base_classes,
                })
            })
            .collect();
        let mut imported: HashMap<String, HashSet<String>> = HashMap::new();
        let mut assigned_attributes = HashSet::new();
        let mut base_classes = HashSet::new();
        for module in modules {
            for (module, name) in module.edges {
                imported.entry(module).or_default().insert(name);
            }
            assigned_attributes.extend(module.assigned_attributes);
            base_classes.extend(module.base_classes);
        }
        for (module, attribute) in entry_points.iter().filter_map(|entry_point| entry_point.split_once(':')) {
            let name = attribute.split('.').next().unwrap_or_default().trim();
            imported.entry(module.trim().to_owned()).or_default().insert(name.to_owned());
        }
        Self { imported, assigned_attributes, base_classes }
    }

    pub fn is_imported(&self, path: &Path, name: &str) -> bool {
        self.imported.get(&module_name(path)).is_some_and(|names| names.contains(name))
    }

    pub fn is_attribute_assigned(&self, name: &str) -> bool {
        self.assigned_attributes.contains(name)
    }

    pub fn is_base_class(&self, name: &str) -> bool {
        self.base_classes.contains(name)
    }
}

struct ModuleFacts {
    edges: Vec<(String, String)>,
    assigned_attributes: Vec<String>,
    base_classes: Vec<String>,
}

struct Imports<'a> {
    package: Vec<String>,
    aliases: HashMap<&'a str, String>,
    edges: Vec<(String, String)>,
    assigned_attributes: Vec<String>,
    base_classes: Vec<String>,
}

pub fn package_parts(path: &Path) -> Vec<String> {
    let mut parts: Vec<String> = std::path::absolute(path)
        .unwrap_or_default()
        .ancestors()
        .skip(1)
        .take_while(|dir| dir.join("__init__.py").is_file())
        .filter_map(|dir| dir.file_name()?.to_str().map(str::to_owned))
        .collect();
    parts.reverse();
    parts
}

impl<'a> Visitor<'a> for Imports<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ImportFrom(ast::StmtImportFrom { module, names, level, .. }) => {
                if let Some(resolved) = self.resolve(module.as_ref().map(ast::Identifier::as_str), *level) {
                    for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                        self.edges.push((resolved.clone(), alias.name.as_str().to_owned()));
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
        match expr {
            Expr::Attribute(ast::ExprAttribute { attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                self.assigned_attributes.push(attr.as_str().to_owned());
            }
            Expr::Attribute(_) => {
                if let Some(path) = self.qualified(expr) {
                    self.record_path(&path);
                    return;
                }
            }
            Expr::StringLiteral(literal) => {
                let text = literal.value.to_str();
                if text.contains('.') && text.split('.').all(Self::is_identifier) {
                    self.record_path(text);
                }
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                if let [target, Expr::StringLiteral(attribute), ..] = &*arguments.args {
                    let attribute = attribute.value.to_str();
                    if func.as_name_expr().is_some_and(|name| name.id.as_str() == "setattr") {
                        self.assigned_attributes.push(attribute.to_owned());
                    }
                    if let Some(module) = self.qualified(target) {
                        self.edges.push((module, attribute.to_owned()));
                    }
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

impl Imports<'_> {
    fn resolve(&self, module: Option<&str>, level: u32) -> Option<String> {
        if level == 0 {
            return module.map(str::to_owned);
        }
        let keep = self.package.len().checked_sub(usize::try_from(level).ok()? - 1)?;
        let mut parts = self.package[..keep].to_vec();
        parts.extend(module.map(str::to_owned));
        Some(parts.join("."))
    }

    fn qualified(&self, expr: &Expr) -> Option<String> {
        let name = UnqualifiedName::from_expr(expr)?;
        let (first, rest) = name.segments().split_first()?;
        let module = self.aliases.get(first)?;
        Some(std::iter::once(module.as_str()).chain(rest.iter().copied()).collect::<Vec<_>>().join("."))
    }

    fn record_path(&mut self, path: &str) {
        let parts: Vec<&str> = path.split('.').collect();
        for split in 1..parts.len() {
            self.edges.push((parts[..split].join("."), parts[split].to_owned()));
        }
    }

    fn is_identifier(part: &str) -> bool {
        part.chars().next().is_some_and(|char| char.is_alphabetic() || char == '_')
            && part.chars().all(|char| char.is_alphanumeric() || char == '_')
    }
}

fn module_name(path: &Path) -> String {
    let mut parts = package_parts(path);
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    if stem != "__init__" {
        parts.push(stem.to_owned());
    }
    parts.join(".")
}
