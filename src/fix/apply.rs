use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

use itertools::Itertools;
use ruff_db::diagnostic::Diagnostic;
use ruff_diagnostics::{Edit, Fix, IsolationLevel};
use ruff_linter::linter::FixTable;
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::types::UnsafeFixes;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashSet;

use crate::lint::registry::{RuleSet, rule_of};

pub(super) struct FixResult {
    pub(super) code: String,
    pub(super) fixes: FixTable,
}

// Ruff's `fix_file` and `apply_fixes`, which `ruff_linter` keeps private: fixes apply in order of their first edit, a
// fix with an edit overlapping or touching one already applied waits for the next pass, and only one fix per isolation
// group applies per pass. Unlike Ruff, the edits of two fixes may interleave: a rename edits every use of a name across
// the module, so with Ruff's rule a module of renames would take one pass each.
pub(super) fn fix_file(
    source: &str,
    diagnostics: &[Diagnostic],
    unsafe_fixes: UnsafeFixes,
    unfixable: RuleSet,
) -> Option<FixResult> {
    let required = unsafe_fixes.required_applicability();
    let candidates = diagnostics
        .iter()
        .filter(|diagnostic| rule_of(diagnostic).is_some_and(|rule| !unfixable.contains(rule)))
        .filter_map(|diagnostic| diagnostic.fix().map(|fix| (diagnostic, fix)))
        .filter(|(_, fix)| fix.applies(required))
        .sorted_by_key(|(_, fix)| fix.min_start())
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    // The accepted edits by range; they never overlap, so only the last one starting at or before an edit's end can
    // reach it.
    let mut accepted: BTreeMap<(TextSize, TextSize), &Edit> = BTreeMap::new();
    let mut isolated: FxHashSet<u32> = FxHashSet::default();
    let mut fixed = FixTable::default();
    for (diagnostic, fix) in candidates {
        let edits: Vec<&Edit> =
            fix.edits().iter().filter(|edit| accepted.get(&(edit.start(), edit.end())) != Some(edit)).collect();
        if edits.iter().any(|edit| {
            accepted
                .range(..=(edit.end(), TextSize::from(u32::MAX)))
                .next_back()
                .is_some_and(|(_, other)| other.end() >= edit.start())
        }) {
            continue;
        }
        if !edits.is_empty()
            && let IsolationLevel::Group(id) = fix.isolation()
            && !isolated.insert(id)
        {
            continue;
        }
        accepted.extend(edits.into_iter().map(|edit| ((edit.start(), edit.end()), edit)));
        *fixed.entry(diagnostic.id()).or_default(diagnostic.secondary_code()) += 1;
    }
    let mut output = String::with_capacity(source.len());
    let mut last = TextSize::default();
    for edit in accepted.into_values() {
        output.push_str(&source[TextRange::new(last, edit.start())]);
        output.push_str(edit.content().unwrap_or_default());
        last = edit.end();
    }
    output.push_str(&source[usize::from(last)..]);
    Some(FixResult { code: output, fixes: fixed })
}

// Ruff's own `fix_file` and `apply_fixes` for `ruff check --fix --unsafe-fixes`, unchanged: unlike simplint's, a fix
// waits for the next pass when an edit of it starts at or before the end of the last edit applied.
pub(super) fn fix_ruff_file<'d>(source: &str, diagnostics: impl Iterator<Item = &'d Diagnostic>) -> Option<FixResult> {
    let mut candidates = diagnostics
        .filter_map(|diagnostic| diagnostic.fix().map(|fix| (diagnostic, fix)))
        .filter(|(_, fix)| fix.applies(UnsafeFixes::Enabled.required_applicability()))
        .sorted_by(|(first, first_fix), (second, second_fix)| {
            ruff_order(first.name(), second.name(), first_fix, second_fix)
        })
        .peekable();
    candidates.peek()?;
    let mut output = String::with_capacity(source.len());
    let mut last: Option<TextSize> = None;
    let mut applied: BTreeSet<&Edit> = BTreeSet::new();
    let mut isolated: FxHashSet<u32> = FxHashSet::default();
    let mut fixed = FixTable::default();
    for (diagnostic, fix) in candidates {
        let edits: Vec<&Edit> = fix.edits().iter().filter(|edit| !applied.contains(edit)).collect();
        if let Some(first) = edits.first()
            && (matches!(fix.isolation(), IsolationLevel::Group(id) if !isolated.insert(id))
                || last.is_some_and(|last| last >= first.start()))
        {
            continue;
        }
        for edit in edits {
            output.push_str(&source[TextRange::new(last.unwrap_or_default(), edit.start())]);
            output.push_str(edit.content().unwrap_or_default());
            last = Some(edit.end());
            applied.insert(edit);
        }
        *fixed.entry(diagnostic.id()).or_default(diagnostic.secondary_code()) += 1;
    }
    output.push_str(&source[usize::from(last.unwrap_or_default())..]);
    Some(FixResult { code: output, fixes: fixed })
}

// Ruff's `cmp_fix`: a redefinition's fix goes first, as removing the unused import can fix it too, then the fixes go in
// order of their first edit, with ties broken for two pairs of rules whose fixes overlap.
fn ruff_order(first: &str, second: &str, first_fix: &Fix, second_fix: &Fix) -> Ordering {
    let redefinition = RuffRule::RedefinedWhileUnused.name().as_str();
    let before = [
        (RuffRule::MissingTrailingPeriod, RuffRule::NewLineAfterLastParagraph),
        (RuffRule::IfElseBlockInsteadOfDictGet, RuffRule::IfElseBlockInsteadOfIfExp),
    ];
    let goes_before = |one: &str, other: &str| {
        before.iter().any(|(earlier, later)| (earlier.name().as_str(), later.name().as_str()) == (one, other))
    };
    (second == redefinition)
        .cmp(&(first == redefinition))
        .then_with(|| first_fix.min_start().cmp(&second_fix.min_start()))
        .then(goes_before(second, first).cmp(&goes_before(first, second)))
}
