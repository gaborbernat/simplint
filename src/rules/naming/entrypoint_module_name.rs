use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::naming::script_targets;
use crate::syntax::is_main_guard;

pub fn entrypoint_module_name(checker: &Checker) {
    let Some(stem @ ("main" | "_main")) = checker.path.file_stem().and_then(|stem| stem.to_str()) else {
        return;
    };
    if !script_targets(checker).is_empty() || checker.body().iter().any(is_main_guard) {
        checker.report(
            Rule::EntrypointModuleName,
            TextRange::default(),
            format!("Entrypoint `{stem}.py` should be named `__main__.py`"),
        );
    }
}
