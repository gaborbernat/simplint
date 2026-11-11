use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::api::{definitions, names_are_followed, references_outside};
use crate::rules::docstrings::Need;
use crate::syntax::{Visibility, module_statements};

pub fn test_only_code(checker: &Checker) {
    let project = checker.project();
    if !names_are_followed(checker) {
        return;
    }
    let api = checker.state().public_api(checker);
    for definition in definitions(module_statements(checker.body())) {
        let name = definition.name.as_str();
        // A decorator may register the definition with something outside the run, such as a plugin list.
        if definition.decorated
            || Visibility::of(name) == Visibility::Dunder
            || api.is_public(definition.stmt.range(), Need::Any)
            || checker.dunder_all().lists(name)
            || project.is_loaded_as_value()
            || !project.is_used_only_by_tests(name)
            || references_outside(checker, name, definition.stmt.range())
        {
            continue;
        }
        checker.report(
            Rule::TestOnlyCode,
            definition.name.range(),
            format!("`{name}` is used only by tests; move it into the tests or a test helper module"),
        );
    }
}
