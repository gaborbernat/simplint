use std::fs;
use std::path::PathBuf;

use ruff_linter::warn_user;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_parser::parse_module;
use rustc_hash::FxHashMap;

use crate::project::facts::qualified;
use crate::syntax::alias_binding;

// The dotted names the `.pth` files at the top of each root make the interpreter load and read at startup, sorted and
// without repeats. `site` runs a line that starts with `import` as Python and reads no other line as code, so a path
// line names nothing. A file that is not UTF-8 is skipped with a warning, as an unreadable Python file is, since `site`
// does not refuse it either.
pub(super) fn names(roots: &[PathBuf]) -> Vec<String> {
    let mut names = Vec::new();
    for root in roots {
        for entry in fs::read_dir(root).into_iter().flatten().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("pth") {
                continue;
            }
            let source = match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    warn_user!("Failed to read `{}`: {error}", path.display());
                    continue;
                }
            };
            names.extend(
                source
                    .lines()
                    .filter(|line| line.starts_with("import ") || line.starts_with("import\t"))
                    .flat_map(line_names),
            );
        }
    }
    names.sort_unstable();
    names.dedup();
    names
}

fn line_names(line: &str) -> Vec<String> {
    let Ok(parsed) = parse_module(line) else { return Vec::new() };
    let mut reads = Reads::default();
    for stmt in &parsed.syntax().body {
        match stmt {
            Stmt::Import(import) => {
                for alias in &import.names {
                    let name = alias.name.as_str();
                    reads.names.push(name.to_owned());
                    let root = name.split_once('.').map_or(name, |(root, _)| root);
                    let (bound, target) = alias.asname.as_ref().map_or((root, root), |asname| (asname.as_str(), name));
                    reads.aliases.insert(bound, target.to_owned());
                }
            }
            Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. }) => {
                let module = module.as_str();
                reads.names.push(module.to_owned());
                for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                    let target = format!("{module}.{}", alias.name.as_str());
                    reads.names.push(target.clone());
                    reads.aliases.insert(alias_binding(alias), target);
                }
            }
            _ => reads.visit_stmt(stmt),
        }
    }
    reads.names
}

#[derive(Default)]
struct Reads<'a> {
    aliases: FxHashMap<&'a str, String>,
    names: Vec<String>,
}

impl<'a> Visitor<'a> for Reads<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match UnqualifiedName::from_expr(expr).and_then(|name| qualified(name.segments(), &self.aliases)) {
            Some(name) => self.names.push(name),
            None => visitor::walk_expr(self, expr),
        }
    }
}
