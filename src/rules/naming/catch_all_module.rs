use std::path::Path;

use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

// Modules named for a grab bag hold whatever does not fit elsewhere, so their names say nothing about their contents.
// A package is named by its folder, so the check reads its `__init__.py` and leaves its modules to their own names.
pub fn catch_all_module(checker: &Checker) {
    let path = checker.path;
    let stem = path.file_stem().and_then(|stem| stem.to_str()).unwrap_or_default();
    let (kind, name) = if stem == "__init__" {
        ("Package", path.parent().and_then(Path::file_name).and_then(|name| name.to_str()))
    } else {
        ("Module", Some(stem))
    };
    let catch_all = &checker.settings.project.knobs.catch_all_modules;
    if let Some(name) = name.filter(|name| catch_all.iter().any(|catch_all| catch_all == name)) {
        checker.report(
            Rule::CatchAllModule,
            TextRange::default(),
            format!("{kind} `{name}` is a catch-all name; name it for the code it holds"),
        );
    }
}
