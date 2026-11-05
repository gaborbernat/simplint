use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::rst::FieldKind;
use rustc_hash::FxHashSet;

pub fn rst_restating_field(checker: &Checker) {
    for target in checker.state().rst_targets(checker) {
        for field in target.docstring.fields(&[FieldKind::Param, FieldKind::Var]) {
            let Some(name) = field.subject().filter(|name| restates(name, &field.description)) else {
                continue;
            };
            checker.report(
                Rule::RstRestatingField,
                field.range_in(&target.docstring),
                format!(
                    "`{}` says only what the name `{name}` does; describe what it holds or how it is used",
                    field.text()
                ),
            );
        }
    }
}

// Whether a description is the name again, as `The user ID.` for `user_id`: every word it has beyond articles and words
// that name what any parameter is is a word of the name.
fn restates(name: &str, description: &str) -> bool {
    const FILLER: &[&str] = &[
        "a",
        "an",
        "the",
        "of",
        "to",
        "for",
        "this",
        "that",
        "given",
        "parameter",
        "argument",
        "value",
        "attribute",
        "variable",
        "instance",
        "object",
    ];
    let name_words: FxHashSet<String> =
        name.split('_').filter(|word| !word.is_empty()).map(str::to_lowercase).collect();
    let words: Vec<String> = description
        .split(|char: char| !char.is_alphanumeric())
        .filter(|word| !word.is_empty())
        .map(str::to_lowercase)
        .filter(|word| !FILLER.contains(&word.as_str()))
        .collect();
    !words.is_empty() && words.iter().all(|word| name_words.contains(word))
}
