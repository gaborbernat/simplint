use ruff_python_ast::PythonVersion;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::{analyze, uses_class_cell};
use crate::rules::classes::{Offer, require_options};

// gh-90562 keeps the `__class__` cell of a slotted dataclass's methods on the class it returns, from CPython 3.14. The
// fix reached 3.13 only in 3.13.14 (gh-105936), and `requires-python = ">=3.13"` admits 3.13.0.
const CLASS_CELL_FIXED: PythonVersion = PythonVersion::PY314;

pub fn dataclass_slots(checker: &Checker) {
    require_options(checker, Rule::DataclassSlots, &["slots"], |decorated, missing| {
        if checker.target_version() < CLASS_CELL_FIXED && uses_class_cell(decorated.class) {
            return Some(Offer::unreported());
        }
        let analysis = analyze(checker, decorated)?;
        Some(Offer::options(
            if analysis.slots_breaks { Vec::new() } else { missing.to_vec() },
            analysis.slots_applicability(),
        ))
    });
}
