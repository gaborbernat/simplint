use std::collections::BTreeSet;

use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::FxHashSet;

use crate::syntax::string_values;

// The words a file's string literals hold, as a run of letters, digits and underscores, which is how a name appears in
// a string that spells it.
pub(super) fn string_words(body: &[Stmt]) -> String {
    let values = string_values(body);
    // A word repeats across the strings, so each is copied once, and sorted once rather than on every insert.
    let words: FxHashSet<&str> = values
        .iter()
        .flat_map(|(_, text)| text.split(|char: char| !(char.is_alphanumeric() || char == '_')))
        .filter(|word| !word.is_empty())
        .collect();
    let mut words: Vec<&str> = words.into_iter().collect();
    words.sort_unstable();
    words.join("\n")
}

// The names the file's `global` statements declare, wherever they stand, which a function rebinds while the module
// runs.
pub(super) fn global_names(body: &[Stmt]) -> Vec<String> {
    let mut names = GlobalNames::default();
    names.visit_body(body);
    names.names.into_iter().collect()
}

#[derive(Default)]
struct GlobalNames {
    names: BTreeSet<String>,
}

impl<'a> Visitor<'a> for GlobalNames {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Global(ast::StmtGlobal { names, .. }) = stmt {
            self.names.extend(names.iter().map(|name| name.as_str().to_owned()));
        }
        visitor::walk_stmt(self, stmt);
    }

    // An expression holds no statement, so no `global` either, and the walk skips them.
    fn visit_expr(&mut self, _: &'a Expr) {}
}
