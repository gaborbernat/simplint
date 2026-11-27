use ruff_python_ast::PythonVersion;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::{analyze, uses_cached_property, uses_class_cell};
use crate::rules::classes::{Offer, require_options};

pub fn dataclass_slots(checker: &Checker) {
    require_options(checker, Rule::DataclassSlots, &["slots"], |decorated, missing| {
        // CPython fixed the __class__ reference in 3.14 and backported it to 3.13.14 (gh-90562 and gh-105936).
        // A py313 target still includes affected patch releases.
        if checker.target_version() < PythonVersion::PY314 && uses_class_cell(decorated.class) {
            return Some(Offer::unreported());
        }
        if uses_cached_property(decorated.class) {
            return Some(Offer::unreported());
        }
        let analysis = analyze(checker, decorated)?;
        Some(Offer::options(
            if analysis.slots_breaks { Vec::new() } else { missing.to_vec() },
            analysis.slots_applicability(),
        ))
    });
}
