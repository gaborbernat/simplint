use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::AutoclassContent;
use crate::rules::docstrings::rst::{FieldKind, Owner};

pub fn rst_duplicate_param(checker: &Checker) {
    let root = checker.settings.project.root.as_path();
    for target in checker.state().rst_targets(checker) {
        let Owner::Class(class) = target.owner else {
            continue;
        };
        let Some(constructor) = &target.constructor else {
            continue;
        };
        let qualified = qualified_name(checker, class.name.as_str());
        let autoclass = checker.project().class_doc_from(root, &qualified).unwrap_or(AutoclassContent::Unknown);
        if autoclass == AutoclassContent::Unknown {
            continue;
        }
        // Under `init` Sphinx renders the `__init__` docstring alone, so the class docstring holds the repeat.
        let (kept, repeated, kept_name, repeated_name) = if autoclass == AutoclassContent::Init {
            (constructor, &target.docstring, "__init__", class.name.as_str())
        } else {
            (&target.docstring, constructor, class.name.as_str(), "__init__")
        };
        let documented: FxHashSet<&str> = kept.subjects(&[FieldKind::Param]).collect();
        for field in repeated
            .fields(&[FieldKind::Param])
            .filter(|field| field.subject().is_some_and(|name| documented.contains(name)))
        {
            let text = field.text();
            checker.report_fix(
                Rule::RstDuplicateParam,
                field.range_in(repeated),
                format!(
                    "`{text}` repeats the docstring of `{kept_name}`, which autodoc shows; document each constructor \
                     parameter once"
                ),
                field.removal(checker, repeated),
                format!("Remove `{text}` from `{repeated_name}`"),
            );
        }
    }
}

// The dotted name the documentation gives the class, under its module's package, as `public_api` builds it.
fn qualified_name(checker: &Checker, class: &str) -> String {
    let module: Vec<&str> = match checker.module_path() {
        Some(path) => path.iter().map(String::as_str).collect(),
        None => vec![checker.file_name().strip_suffix(".py").unwrap_or(checker.file_name())],
    };
    module.into_iter().filter(|part| *part != "__init__").chain([class]).collect::<Vec<_>>().join(".")
}
