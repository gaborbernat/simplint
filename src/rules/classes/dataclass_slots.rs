use std::cell::OnceCell;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::analyze;
use crate::rules::classes::{Offer, require_options};

pub fn dataclass_slots(checker: &Checker) {
    let expected_to_raise = OnceCell::new();
    require_options(checker, Rule::DataclassSlots, &["slots"], |decorated, missing| {
        let analysis = analyze(checker, decorated, &expected_to_raise)?;
        let options = if analysis.slots_breaks { Vec::new() } else { missing.to_vec() };
        Some(Offer {
            options,
            edits: Vec::new(),
            applicability: analysis.slots_applicability(),
            related: Vec::new(),
            note: String::new(),
            left_to_ruff: false,
        })
    });
}
