use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::imports::stdlib::ships_on_some_version;
use crate::settings::{Settings, canonical_name, may_provide};

// The package initializer gives a project-level diagnostic a Python location where it can be suppressed.
pub fn unused_dependency(checker: &Checker) {
    let Some(imported) = checker.project().imported_modules() else {
        return;
    };
    let metadata = &checker.settings.project.metadata;
    let imported: Vec<&str> =
        imported.iter().map(String::as_str).filter(|module| !ships_on_some_version(module)).collect();
    for (name, modules) in &metadata.requirements {
        if metadata.name.as_deref() == Some(canonical_name(name).as_str())
            || imported.iter().any(|module| may_provide(modules, module))
        {
            continue;
        }
        checker.report(
            Rule::UnusedDependency,
            TextRange::default(),
            format!("`{name}` is a requirement of the project, but nothing outside the tests imports it; remove it"),
        );
    }
}

// A nested project may be test data with deliberately unused requirements.
pub(crate) fn is_dependency_host(settings: &Settings, module_path: Option<&[String]>, is_test: bool) -> bool {
    let host = settings.api.packages.iter().flatten().find(|package| !package.contains('.'));
    settings.metadata.lists_dependencies
        && !is_test
        && !settings.nested
        && host.zip(module_path).is_some_and(|(host, module_path)| module_path == [host.as_str(), "__init__"])
}
