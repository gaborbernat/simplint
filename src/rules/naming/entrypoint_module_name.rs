use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::is_main_guard;

pub fn entrypoint_module_name(checker: &Checker) {
    let Some(stem @ ("main" | "_main")) = checker.path.file_stem().and_then(|stem| stem.to_str()) else {
        return;
    };
    let module = checker.module_path().map(|path| path.join(".")).or_else(|| {
        let path = checker.path.strip_prefix(&checker.settings.project.root).ok()?;
        let path = path.strip_prefix("src").unwrap_or(path);
        let parts = path.with_extension("");
        Some(parts.iter().map(|part| part.to_str()).collect::<Option<Vec<_>>>()?.join("."))
    });
    let script = module.is_some_and(|module| {
        checker
            .settings
            .project
            .metadata
            .scripts
            .iter()
            .any(|entry| entry.split_once(':').is_some_and(|(entry_module, _)| entry_module == module))
    });
    if script || checker.body().iter().any(is_main_guard) {
        checker.report(
            Rule::EntrypointModuleName,
            TextRange::default(),
            format!("Entrypoint `{stem}.py` should be named `__main__.py`"),
        );
    }
}
