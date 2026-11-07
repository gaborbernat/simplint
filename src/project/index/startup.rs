use std::fs;
use std::path::PathBuf;

use ruff_linter::warn_user;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{Expr, Stmt};
use ruff_python_parser::parse_module;
use rustc_hash::FxHashMap;

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

// The names one import line reads: the modules it imports, and the dotted names of the attributes its statements read
// off the modules it binds. A line that does not parse makes the interpreter fail at startup, so it names nothing here.
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
                    reads.bound.insert(bound.to_owned(), target.to_owned());
                }
            }
            _ => reads.visit_stmt(stmt),
        }
    }
    reads.names
}

// The module each name an import line binds stands for, and the dotted names the line reads off them.
#[derive(Default)]
struct Reads {
    bound: FxHashMap<String, String>,
    names: Vec<String>,
}

impl Reads {
    // The dotted name an expression reads off a bound module, as `pkg.hook._run` does in `pkg.hook._run()`.
    fn dotted(&self, expr: &Expr) -> Option<String> {
        match expr {
            Expr::Name(name) => self.bound.get(name.id.as_str()).cloned(),
            Expr::Attribute(attribute) => Some(format!("{}.{}", self.dotted(&attribute.value)?, attribute.attr)),
            _ => None,
        }
    }
}

impl<'a> Visitor<'a> for Reads {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match self.dotted(expr) {
            Some(name) => self.names.push(name),
            None => visitor::walk_expr(self, expr),
        }
    }
}
