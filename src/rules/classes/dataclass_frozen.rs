use std::cell::OnceCell;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::analyze;
use crate::rules::classes::{Offer, require_options};

pub fn dataclass_frozen(checker: &Checker) {
    let expected_to_raise = OnceCell::new();
    require_options(checker, Rule::DataclassFrozen, &["frozen"], |decorated| {
        let analysis = analyze(checker, decorated, &expected_to_raise)?;
        let options = if analysis.frozen_breaks { Vec::new() } else { decorated.missing.clone() };
        Some(Offer { options, edits: Vec::new(), applicability: analysis.applicability() })
    });
}
