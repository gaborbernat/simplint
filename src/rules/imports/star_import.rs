use ruff_diagnostics::{Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::{self as ast, Stmt};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::Ranged;
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::{FileIndex, StarSite};
use crate::syntax::{Exports, Visibility};

pub fn star_import(checker: &Checker) {
    let mut imports = Vec::new();
    star_imports(checker.body(), false, &mut imports);
    if imports.is_empty() {
        return;
    }
    // A module whose `__all__` is computed, often from the star-imported module's own, re-exports what it imports.
    let listed_here: FxHashSet<&str> = match checker.dunder_all() {
        Exports::Listed(names) => names.iter().copied().collect(),
        Exports::Absent => FxHashSet::default(),
        Exports::Dynamic => return,
    };
    let project = checker.project();
    let target_version = checker.settings.from_ruff.target_version.minor;
    let read: FxHashSet<&str> = checker
        .semantic()
        .unresolved_references()
        .map(|range| checker.text(range))
        .filter(|name| !is_python_builtin(name, target_version, false))
        .collect();
    let file = File { checker, project: &project, read: &read, listed_here: &listed_here };
    // Each fix replaces the one star import of its file, so a file with several keeps them all.
    let sole = imports.len() == 1;
    for (import, fallback) in imports {
        let Some(site) = project.star_site(import.range()) else {
            continue;
        };
        // A star import in an `except` clause stands in for the one its `try` attempts, so it must take the same names,
        // and one the file reads nothing from is there for the names it re-exports.
        if fallback || !file.uses(site) {
            continue;
        }
        let module =
            format!("{}{}", ".".repeat(import.level as usize), import.module.as_ref().map_or("", |name| name.as_str()));
        let fix = sole.then(|| file.fix(import, site)).flatten();
        // Ruff's F403 reports every star import, and this rule leaves the ones it cannot fix to it while it runs.
        let unfixable = checker.settings.project.fix_for(Rule::StarImport, fix.clone()).is_none();
        if unfixable && checker.settings.from_ruff.ruff_runs(RuffRule::UndefinedLocalWithImportStar) {
            continue;
        }
        let title = format!("Import the names this file uses from `{module}`");
        checker.report_fix(
            Rule::StarImport,
            import.range(),
            format!("`from {module} import *` hides which names this file takes from `{module}`; import them by name"),
            fix,
            title,
        );
    }
}

// The `from module import *` statements of the module level, each with whether it sits in an `except` clause.
fn star_imports<'a>(body: &'a [Stmt], fallback: bool, found: &mut Vec<(&'a ast::StmtImportFrom, bool)>) {
    for stmt in body {
        match stmt {
            Stmt::ImportFrom(import) if import.names.iter().any(|alias| alias.name.as_str() == "*") => {
                found.push((import, fallback));
            }
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                star_imports(body, fallback, found);
                for clause in elif_else_clauses {
                    star_imports(&clause.body, fallback, found);
                }
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                star_imports(body, fallback, found);
                for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                    star_imports(&handler.body, true, found);
                }
                star_imports(orelse, fallback, found);
                star_imports(finalbody, fallback, found);
            }
            _ => {}
        }
    }
}

// What the file takes from star imports: the names it reads that no binding of its own or builtin defines, and the
// names its own `__all__` lists.
struct File<'c, 'a> {
    checker: &'c Checker<'a>,
    project: &'c FileIndex<'a>,
    read: &'c FxHashSet<&'a str>,
    listed_here: &'c FxHashSet<&'a str>,
}

impl File<'_, '_> {
    // Whether the file reads or lists a name the module lists or defines.
    fn uses(&self, site: &StarSite) -> bool {
        let exports = site.exports();
        let mut known = exports.defined().iter().chain(exports.listed().into_iter().flatten());
        known.any(|name| self.read.contains(name.as_str()) || self.listed_here.contains(name.as_str()))
    }

    // The star import with the names this file reads from the module written in its place. The fix is safe when the
    // module lists its exports in a static `__all__`; without one, the names it defines are the only ones known, and
    // the fix is unsafe, as the module may export names it imports too. Either way every name the file reads must be
    // among them: the module may add a name to `__all__` where it cannot be read, as inside an `if` block.
    //
    // The fix leaves the import alone when another file star-imports this one, when the module's exports cannot be
    // read, when a comment on or above the import may say why it is a star import, and in an `__init__.py` without a
    // static `__all__`, whose star imports make up its package's names.
    fn fix(&self, import: &ast::StmtImportFrom, site: &StarSite) -> Option<Fix> {
        let (checker, project) = (self.checker, self.project);
        let exports = site.exports();
        if exports.unknown()
            || project.is_star_import_source()
            || checker.has_comment_in(import.range())
            || checker.comment_above(import).is_some()
            || (checker.is_init_module() && self.listed_here.is_empty())
        {
            return None;
        }
        let (provided, safe): (FxHashSet<&str>, bool) = match exports.listed() {
            Some(listed) => (listed.iter().map(String::as_str).collect(), true),
            None => (
                exports
                    .defined()
                    .iter()
                    .map(String::as_str)
                    .filter(|name| Visibility::of(name) == Visibility::Public)
                    .collect(),
                false,
            ),
        };
        if !self.read.iter().all(|name| provided.contains(name)) {
            return None;
        }
        // A name the file lists in its own `__all__` stays bound, as does one it reads. A name only other files or
        // library users reach through this one would be an import the file never uses, which Ruff's F401 removes, so
        // the fix declines instead.
        let mut names: Vec<&str> = Vec::new();
        for name in provided {
            if self.read.contains(name) || self.listed_here.contains(name) {
                names.push(name);
            } else if project.is_referenced(name) || project.user_facing(name).is_some() {
                return None;
            }
        }
        if names.is_empty() {
            return None;
        }
        names.sort_unstable();
        let edit = Edit::range_replacement(names.join(", "), import.names[0].range());
        Some(if safe { Fix::safe_edit(edit) } else { Fix::unsafe_edit(edit) })
    }
}
