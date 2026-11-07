use std::cell::OnceCell;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::analyze;
use crate::rules::classes::{Offer, library_api, require_options};

pub fn dataclass_frozen(checker: &Checker) {
    // Users of a library's public dataclass construct and subclass it, which this option changes.
    let expected_to_raise = OnceCell::new();
    require_options(checker, Rule::DataclassFrozen, &["frozen"], library_api(checker), |decorated| {
        let analysis = analyze(checker, decorated, &expected_to_raise)?;
        let options = if analysis.frozen_breaks { Vec::new() } else { decorated.missing.clone() };
        Some(Offer {
            options,
            edits: Vec::new(),
            applicability: analysis.applicability(),
            related: Vec::new(),
            note: String::new(),
        })
    });
}
