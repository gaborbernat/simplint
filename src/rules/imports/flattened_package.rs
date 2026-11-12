use ruff_text_size::TextRange;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::name_prefix;

pub fn flattened_package(checker: &Checker) {
    let (Some(siblings), Some(prefix)) = (
        checker.project().prefix_siblings(),
        checker.path.file_stem().and_then(|stem| stem.to_str()).and_then(name_prefix),
    ) else {
        return;
    };
    let file = checker.file_name();
    let others = siblings.others;
    let message = match siblings.prefix_module {
        Some(extension) => format!(
            "`{file}` shares the `_{prefix}_` prefix with {others} other modules; move `_{prefix}.{extension}` into a \
             `_{prefix}` package as its `__init__.py`, which re-exports their names"
        ),
        None => format!(
            "`{file}` shares the `_{prefix}_` prefix with {others} other modules; group them in a `_{prefix}` package \
             that re-exports their names from its `__init__.py`"
        ),
    };
    checker.report(Rule::FlattenedPackage, TextRange::default(), message);
}
