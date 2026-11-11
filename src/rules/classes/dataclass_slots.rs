use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::classes::dataclass_hazards::analyze;
use crate::rules::classes::{Offer, require_options};

pub fn dataclass_slots(checker: &Checker) {
    require_options(checker, Rule::DataclassSlots, &["slots"], |decorated, missing| {
        let analysis = analyze(checker, decorated)?;
        Some(Offer::options(
            if analysis.slots_breaks { Vec::new() } else { missing.to_vec() },
            analysis.slots_applicability(),
        ))
    });
}
