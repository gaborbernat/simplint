use std::borrow::Cow;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::Path;

use ruff_python_ast::helpers::resolve_imported_module_path;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_parser::parse_module;
use ruff_python_stdlib::identifiers::is_identifier;

// The names a file uses from other modules, before they resolve to files; parsing finds them and the cache keeps them.
// Names are grouped by module, so each module resolves once per file.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct FileFacts {
    pub(super) imports: Vec<ModuleNames>,
    pub(super) references: Vec<ModuleNames>,
    pub(super) declared: Vec<String>,
    pub(super) assigned_attributes: Vec<String>,
    pub(super) base_classes: Vec<String>,
}

impl FileFacts {
    // The module path, such as `["package", "module"]`, is what relative imports resolve against; a file outside any
    // package has none, so its relative imports resolve to nothing.
    pub fn collect(path: &Path, module_path: Option<&[String]>) -> Option<Self> {
        let source = std::fs::read_to_string(path).ok()?;
        let parsed = parse_module(&source).ok()?;
        let mut imports = Imports {
            module_path,
            aliases: HashMap::new(),
            edges: BTreeMap::new(),
            references: BTreeMap::new(),
            assigned_attributes: Vec::new(),
            base_classes: Vec::new(),
        };
        imports.visit_body(&parsed.syntax().body);
        // A stub publishes what it declares, so its names are the API of the module it describes.
        let declared = if path.extension().is_some_and(|extension| extension == "pyi") {
            parsed.syntax().body.iter().flat_map(declared_names).collect()
        } else {
            Vec::new()
        };
        Some(Self {
            imports: grouped(imports.edges),
            references: grouped(imports.references),
            declared,
            assigned_attributes: imports.assigned_attributes,
            base_classes: imports.base_classes,
        })
    }
}

// A module and the names used from it.
pub(super) type ModuleNames = (String, Vec<String>);

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
}

impl<'a> Visitor<'a> for Imports<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ImportFrom(ast::StmtImportFrom { module, names, level, .. }) => {
                let module = module.as_ref().map(ast::Identifier::as_str);
                if let Some(resolved) =
                    resolve_imported_module_path(*level, module, self.module_path).map(Cow::into_owned)
                {
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
