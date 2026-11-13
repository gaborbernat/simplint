use ruff_text_size::TextRange;

use crate::rules::imports::stdlib::ships_on_some_version;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::settings::may_provide;

// The project has no Python file of its own for `pyproject.toml`, so the `__init__.py` of its first top-level package
// holds the reports, at its start.
pub fn unused_dependency(checker: &Checker) {
    let settings = &checker.settings.project;
    let metadata = &settings.metadata;
    let (Some(packages), Some(module_path)) = (&settings.api.packages, checker.module_path()) else {
        return;
    };
    let Some(host) = packages.iter().find(|package| !package.contains('.')) else {
        return;
    };
    if !metadata.lists_dependencies || checker.is_test_module() || module_path != [host.as_str(), "__init__"] {
        return;
    }
    let Some(imported) = checker.project().imported_modules() else {
        return;
    };
    let imported: Vec<&str> = imported.iter().map(String::as_str).filter(|module| !ships_on_some_version(module)).collect();
    for (name, modules) in &metadata.requirements {
        if metadata.name.as_deref() == Some(name.as_str()) || imported.iter().any(|module| may_provide(modules, module)) {
            continue;
        }
        checker.report(
            Rule::UnusedDependency,
            TextRange::default(),
            format!("`{name}` is a requirement of the project, but nothing outside the tests imports it; remove it"),
        );
    }
}
