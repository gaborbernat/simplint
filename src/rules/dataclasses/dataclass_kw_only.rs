use ruff_diagnostics::Applicability;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::dataclasses::require_options;

pub fn dataclass_kw_only(checker: &Checker) {
    require_options(checker, Rule::DataclassKwOnly, &["kw_only"], |_, _, _| Applicability::Unsafe);
}
