use std::collections::HashSet;

use ruff_diagnostics::{Applicability, Fix, IsolationLevel};
use ruff_text_size::{Ranged, TextSize};

use crate::checker::Diagnostic;

pub fn apply(source: &str, diagnostics: &[Diagnostic], applicability: Applicability) -> Option<(String, usize)> {
    let mut fixes: Vec<&Fix> = diagnostics
        .iter()
        .filter_map(|diagnostic| diagnostic.fix.as_ref())
        .filter(|fix| fix.applies(applicability))
        .collect();
    fixes.sort_by_key(|fix| fix.min_start());
    let mut output = String::with_capacity(source.len());
    let mut last = TextSize::default();
    let mut groups = HashSet::new();
    let mut applied: Vec<&Fix> = Vec::new();
    for fix in fixes {
        if applied.contains(&fix) {
            continue;
        }
        let mut edits = fix.edits().to_vec();
        edits.sort_by_key(|edit| (edit.start(), edit.end()));
        if edits.first().is_some_and(|edit| edit.start() < last)
            || edits.windows(2).any(|pair| pair[1].start() < pair[0].end())
            || matches!(fix.isolation(), IsolationLevel::Group(group) if !groups.insert(group))
        {
            continue;
        }
        for edit in edits {
            output.push_str(&source[usize::from(last)..usize::from(edit.start())]);
            output.push_str(edit.content().unwrap_or_default());
            last = edit.end();
        }
        applied.push(fix);
    }
    if applied.is_empty() {
        return None;
    }
    output.push_str(&source[usize::from(last)..]);
    Some((output, applied.len()))
}
