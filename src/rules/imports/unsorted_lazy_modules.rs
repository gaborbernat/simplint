use ruff_diagnostics::Fix;
use ruff_text_size::Ranged;

use super::lazy_modules::{Declaration, declaration, sort_key};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn unsorted_lazy_modules(checker: &Checker) {
    let Declaration::Listed(list) = declaration(checker) else {
        return;
    };
    if list
        .entries
        .iter()
        .map(|entry| checker.text(entry.expr))
        .is_sorted_by(|left, right| sort_key(left) <= sort_key(right))
    {
        return;
    }
    let fix = list
        .items(checker)
        .and_then(|mut items| {
            items.sort_by(|left, right| sort_key(&left.text).cmp(&sort_key(&right.text)));
            list.replacement(checker, &items)
        })
        .map(Fix::safe_edit);
    checker.report_fix(
        Rule::UnsortedLazyModules,
        list.value.range(),
        "`__lazy_modules__` is not sorted; list plain names first, then the `__spec__.parent` entries",
        fix,
        "Sort `__lazy_modules__`",
    );
}
