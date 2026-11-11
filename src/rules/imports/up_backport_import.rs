use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Blanks, statement_removal};
use crate::rules::is_alone_on_lines;
use crate::syntax::for_each_body;

enum Replacement {
    // The standard library module keeps the backport's API, so only the module name changes.
    Module(&'static str),
    // The builtins define these names from the version on, so the import goes.
    Builtins(&'static [&'static str]),
    // The standard library moved the API, so each use needs a review.
    Review(&'static str),
}

struct Backport {
    package: &'static str,
    since: PythonVersion,
    replacement: Replacement,
}

const BACKPORTS: &[Backport] = &[
    Backport { package: "tomli", since: PythonVersion::PY311, replacement: Replacement::Module("tomllib") },
    Backport {
        package: "exceptiongroup",
        since: PythonVersion::PY311,
        replacement: Replacement::Builtins(&["ExceptionGroup", "BaseExceptionGroup"]),
    },
    Backport {
        package: "async_timeout",
        since: PythonVersion::PY311,
        replacement: Replacement::Review("asyncio.timeout"),
    },
    Backport {
        package: "taskgroup",
        since: PythonVersion::PY311,
        replacement: Replacement::Review("asyncio.TaskGroup"),
    },
    Backport {
        package: "importlib_metadata",
        since: PythonVersion::PY310,
        replacement: Replacement::Review("importlib.metadata"),
    },
    Backport {
        package: "importlib_resources",
        since: PythonVersion::PY310,
        replacement: Replacement::Review("importlib.resources"),
    },
];

pub fn up_backport_import(checker: &Checker) {
    let target = checker.target_version();
    for_each_body(checker.body(), &mut |statements, owner| {
        // A guarded import is a fallback, which the import guard rule reviews, so it is left alone here.
        if matches!(owner, Some(Stmt::Try(_))) {
            return;
        }
        for stmt in statements {
            match stmt {
                Stmt::Import(import) => {
                    for alias in &import.names {
                        let Some(backport) = backport_named(alias.name.as_str(), target) else {
                            continue;
                        };
                        if matches!(backport.replacement, Replacement::Builtins(_)) {
                            continue;
                        }
                        // Without an alias the name the module binds changes too, and its uses would need renaming.
                        let fix = match backport.replacement {
                            Replacement::Module(module) if alias.asname.is_some() => {
                                Some(Fix::safe_edit(Edit::range_replacement(module.to_owned(), alias.name.range())))
                            }
                            _ => None,
                        };
                        report(checker, alias.range(), backport, fix);
                    }
                }
                Stmt::ImportFrom(import @ ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
                    let Some(backport) = backport_named(module.as_str(), target) else {
                        continue;
                    };
                    let fix = match backport.replacement {
                        Replacement::Module(replacement) => {
                            Some(Fix::safe_edit(Edit::range_replacement(replacement.to_owned(), module.range())))
                        }
                        Replacement::Builtins(names) => {
                            // A name the builtins lack, such as `catch`, still comes from the backport.
                            if !import.names.iter().all(|alias| names.contains(&alias.name.as_str())) {
                                continue;
                            }
                            let unaliased = import.names.iter().all(|alias| alias.asname.is_none());
                            // Another module that imports the name from this one, or `__all__`, would lose it.
                            let exported = import.names.iter().any(|alias| checker.is_exported(alias.name.as_str()));
                            (unaliased && !exported && is_alone_on_lines(checker.source, stmt.range()))
                                .then(|| statement_removal(checker, stmt.range(), statements.len(), None, Blanks::Keep))
                                .flatten()
                                .map(Fix::safe_edit)
                        }
                        Replacement::Review(_) => None,
                    };
                    report(checker, import.range(), backport, fix);
                }
                _ => {}
            }
        }
    });
}

fn backport_named(module: &str, target: PythonVersion) -> Option<&'static Backport> {
    BACKPORTS.iter().find(|backport| backport.package == module && target >= backport.since)
}

fn report(checker: &Checker, range: TextRange, backport: &Backport, fix: Option<Fix>) {
    let package = backport.package;
    let (message, title) = match backport.replacement {
        Replacement::Module(module) => (
            format!(
                "`{package}` backports `{module}`, which every supported Python version ships; import `{module}` \
                 and drop `{package}` from the dependencies"
            ),
            format!("Import `{module}` instead"),
        ),
        Replacement::Builtins(_) => (
            format!("`{package}` names are builtins in every supported Python version; drop the import"),
            "Remove the import".to_owned(),
        ),
        Replacement::Review(replacement) => (
            format!(
                "`{package}` backports `{replacement}`, which every supported Python version ships; check the calls, \
                 as the API may differ"
            ),
            "Review the calls".to_owned(),
        ),
    };
    checker.report_fix(Rule::BackportImport, range, message, fix, title);
}
