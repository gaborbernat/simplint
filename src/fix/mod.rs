mod apply;
mod cleanup;
mod errors;

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::Result;
use rayon::prelude::*;
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::linter::FixTable;
use ruff_linter::settings::flags;
use ruff_linter::settings::types::UnsafeFixes;
use ruff_python_parser::{UnsupportedSyntaxError, UnsupportedSyntaxErrorKind};
use ruff_text_size::TextRange;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::fix::apply::{FixResult, fix_file};
use crate::fix::cleanup::cleanup;
use crate::lint::registry::{Rule, RuleSet, rule_of};
use crate::lint::ruff::Checked;
use crate::lint::{Lint, Linter};

pub struct FixerResult {
    pub diagnostics: Vec<Diagnostic>,
    pub transformed: String,
    pub fixed: FixTable,
}

// A source and the Ruff rules, sorted, whose findings a cleanup of it fixes.
type CleanupKey = (Arc<str>, Vec<&'static str>);

// One file's fix loop. The withdrawal candidates often reach the same code, so each source is linted once, by simplint
// and by Ruff, and each cleanup runs once; the Ruff passes of a cleanup are the same runs that count Ruff's findings.
pub struct Fixer<'l, 'a> {
    linter: &'l Linter<'a>,
    path: &'l Path,
    unsafe_fixes: UnsafeFixes,
    noqa: flags::Noqa,
    sources: Sources,
    lints: Memo<Lint>,
    checks: Memo<Arc<Checked>>,
    failing: Memo<Arc<FxHashSet<&'static str>>>,
    cleanups: Mutex<FxHashMap<CleanupKey, Arc<str>>>,
}

impl<'l, 'a> Fixer<'l, 'a> {
    pub fn new(linter: &'l Linter<'a>, path: &'l Path, unsafe_fixes: UnsafeFixes, noqa: flags::Noqa) -> Self {
        Self {
            linter,
            path,
            unsafe_fixes,
            noqa,
            sources: Sources::default(),
            lints: Memo::default(),
            checks: Memo::default(),
            failing: Memo::default(),
            cleanups: Mutex::default(),
        }
    }

    // Fixes run after `ruff check --fix --unsafe-fixes`, so their result must not give it new work: a Ruff finding the
    // original file did not have gets Ruff's own fix when Ruff has one, and otherwise the fixes that caused it are
    // withdrawn, one report at a time, keeping the other fixes of their rule and the reports. `ruff format`, which runs
    // last, keeps the syntax tree, so it cannot undo this.
    //
    // A run can withdraw a fix that is only bad next to another rule's pending fixes, yet once those apply, a run over
    // the result can offer the withdrawn fixes again. The single fixes withdrawn carry over to the later runs, which then
    // do not try them again; whole rules do not, as a rule may cover reports that only appear once other fixes apply.
    // Running until a run changes nothing, or returns to a source seen before, makes a single run final.
    pub fn fix(&self, source: &str) -> Result<(FixerResult, Withdrawn)> {
        let mut fixed = FixTable::default();
        let mut seen = FxHashSet::from_iter([source.to_owned()]);
        let mut current = source.to_owned();
        let mut withdrawn = Withdrawn::default();
        let mut result = loop {
            let (result, run_withdrawn) = self.run(&current, std::mem::take(&mut withdrawn.fixes))?;
            withdrawn = run_withdrawn;
            add_fixes(&mut fixed, &result.fixed);
            if result.fixed.is_empty() || !seen.insert(result.transformed.clone()) {
                break result;
            }
            errors::ensure_converging(self.path, seen.len(), &result.diagnostics)?;
            current.clone_from(&result.transformed);
        };
        result.fixed = fixed;
        let (mut result, withdrawn) = self.unless_unchanged(source, result, withdrawn);
        Self::withdraw(&result.transformed, &mut result.diagnostics, &withdrawn);
        Ok((result, withdrawn))
    }

    // A withdrawn fix is no longer offered, so the summary does not count it as fixable.
    pub fn withdraw(source: &str, diagnostics: &mut [Diagnostic], withdrawn: &Withdrawn) {
        let held = withdrawn.held_back(source, diagnostics);
        for (diagnostic, held) in diagnostics.iter_mut().zip(held) {
            if held {
                diagnostic.remove_fix();
            }
        }
    }

    fn run(&self, source: &str, fixes: FxHashSet<FixKey>) -> Result<(FixerResult, Withdrawn)> {
        let mut withdrawn = Withdrawn { rules: RuleSet::default(), fixes };
        let linted = Linted { source, lint: self.lint(source) };
        let mut settled = self.settle(&linted, &withdrawn)?;
        while settled.left > 0 {
            // Rules are tried in code order, so the same file always yields the same fixes. Among withdrawals that leave
            // as little for Ruff, one whose result a second run leaves alone wins, so a single run is final. The
            // candidates run in parallel and are compared in that order, which keeps the choice the same.
            let applied: BTreeSet<Rule> = Self::fixing_rules(&settled.result.fixed).collect();
            let fewest = applied
                .par_iter()
                .map(|&rule| {
                    let batch = withdrawn.with_rule(rule);
                    let settled = self.settle(&linted, &batch)?;
                    let reapplies = settled.left == 0 && self.reapplies(&settled.result.transformed)?;
                    Ok(Candidate { rule, withdrawn: batch, reapplies, settled })
                })
                .reduce_with(earlier_unless_beaten)
                .transpose()?;
            // Every fix comes from a simplint rule, so a fix that left Ruff work has a rule to withdraw.
            let Candidate { rule, withdrawn: batch, settled: batch_settled, .. } =
                fewest.expect("an applied fix names its rule");
            // Withdrawing a rule's batch that leaves less Ruff work points at its fixes, so they are judged one by one.
            let split =
                if batch_settled.left < settled.left { self.withdraw_fixes(&linted, &withdrawn, rule)? } else { None };
            (withdrawn, settled) = match split {
                Some(split) => split,
                None => (batch, batch_settled),
            };
        }
        Ok(self.unless_unchanged(source, settled.result, settled.held))
    }

    // Fixes that leave the source as it was, as when Ruff's cleanup undoes them, such as a relative import the project
    // bans, or when later runs of the loop return to the source, fixed nothing. They are withdrawn, so their reports
    // are not offered as fixable and the file is not written.
    fn unless_unchanged(&self, source: &str, result: FixerResult, mut withdrawn: Withdrawn) -> (FixerResult, Withdrawn) {
        if result.transformed != source || result.fixed.is_empty() {
            return (result, withdrawn);
        }
        withdrawn.rules = withdrawn.rules.union(RuleSet::from_rules(Self::fixing_rules(&result.fixed)));
        let diagnostics = self.lint(source).diagnostics;
        (FixerResult { diagnostics, transformed: source.to_owned(), fixed: FixTable::default() }, withdrawn)
    }

    // Withdraws the fixes of `rule` that `linted` reports, keeping each group of them whose result leaves no more Ruff
    // work than the result without it. `None` when none is withdrawn, as when the rule's fixes come from a later pass,
    // so the caller withdraws the whole rule instead.
    fn withdraw_fixes(
        &self,
        linted: &Linted,
        withdrawn: &Withdrawn,
        rule: Rule,
    ) -> Result<Option<(Withdrawn, Settled)>> {
        let keys: Vec<FixKey> = fix_keys(linted.source, &linted.lint.diagnostics)
            .into_iter()
            .map(|(_, key)| key)
            .filter(|key| key.0 == rule && !withdrawn.fixes.contains(key))
            .collect();
        let mut withheld = withdrawn.clone();
        withheld.fixes.extend(keys.iter().cloned());
        let settled = self.settle(linted, &withheld)?;
        let mut admission = Admission { withheld, settled };
        self.admit(linted, &keys, &mut admission)?;
        let Admission { withheld, settled } = admission;
        Ok((withheld.fixes.len() > withdrawn.fixes.len()).then_some((withheld, settled)))
    }

    // Enables `keys` together when that adds no Ruff work to the admission. Otherwise it halves them, so each bad fix
    // costs about log N settles rather than one per fix, and a single fix that adds work stays withheld.
    fn admit(&self, linted: &Linted, keys: &[FixKey], admission: &mut Admission) -> Result<()> {
        let mut enabled = admission.withheld.clone();
        for key in keys {
            enabled.fixes.remove(key);
        }
        let settled = self.settle(linted, &enabled)?;
        if settled.left <= admission.settled.left {
            *admission = Admission { withheld: enabled, settled };
        } else if let [_, _, ..] = keys {
            let (first, second) = keys.split_at(keys.len() / 2);
            self.admit(linted, first, admission)?;
            self.admit(linted, second, admission)?;
        }
        Ok(())
    }

    fn fixing_rules(fixes: &FixTable) -> impl Iterator<Item = Rule> {
        fixes.iter().filter_map(|(_, code, _)| Rule::from_code(code?.as_str()))
    }

    // Whether another run over the fixed source would apply fixes and leave Ruff nothing new.
    fn reapplies(&self, source: &str) -> Result<bool> {
        let settled = self.settle(&Linted { source, lint: self.lint(source) }, &Withdrawn::default())?;
        Ok(!settled.result.fixed.is_empty() && settled.result.transformed != source && settled.left == 0)
    }

    // The fixed result and how many Ruff rules still find more in it than in the original. Ruff's cleanup can give the
    // rules new work, as dropping a branch the target version always takes can leave a constant one function reads, so
    // fixes and cleanups alternate until the fixes bring Ruff nothing new, or the cleanup returns to code it already
    // gave, as when it undoes a fix.
    fn settle(&self, linted: &Linted, withdrawn: &Withdrawn) -> Result<Settled> {
        let mut seen = FxHashSet::from_iter([linted.source.to_owned()]);
        let mut fixed = FixTable::default();
        let mut held = withdrawn.clone();
        let mut round = self.fix_with(linted, &mut held)?;
        loop {
            if round.fixed.is_empty() {
                return Ok(Settled { result: FixerResult { fixed, ..round }, left: 0, held });
            }
            add_fixes(&mut fixed, &round.fixed);
            let Some((transformed, left)) = self.clean_up(linted.source, &round.transformed) else {
                return Ok(Settled { result: FixerResult { fixed, ..round }, left: 0, held });
            };
            let lint = self.lint(&transformed);
            if left > 0 || !seen.insert(transformed.clone()) {
                let result = FixerResult { diagnostics: lint.diagnostics, transformed, fixed };
                return Ok(Settled { result, left, held });
            }
            errors::ensure_converging(self.path, seen.len(), &lint.diagnostics)?;
            round = self.fix_with(&Linted { source: &transformed, lint }, &mut held)?;
        }
    }

    // Ruff's fixes for the findings `transformed` has beyond `start`, and how many of its rules still find more there;
    // `None` when it has none. The Ruff findings of `start` are only needed once a fix applied, so a file without fixes
    // never runs Ruff.
    fn clean_up(&self, start: &str, transformed: &str) -> Option<(String, usize)> {
        let baseline = self.check(start);
        let failing = || {
            self.failing.get_or(&self.sources, start, || Arc::new(self.linter.ruff.fixable_at_limit(self.path, start)))
        };
        let mut introduced = self.check(transformed).findings.introduced_since(&baseline.findings, failing);
        if introduced.is_empty() {
            return None;
        }
        // Ruff's fixes can give other rules new work, as moving an import to the top unsorts the imports there, so the
        // cleanup takes those rules in too until it brings no new one.
        let mut cleaned = self.cleanup(transformed, &introduced);
        loop {
            let left = self.check(&cleaned).findings.introduced_since(&baseline.findings, failing);
            if left.is_subset(&introduced) {
                return Some((cleaned, left.len()));
            }
            introduced.extend(left);
            cleaned = self.cleanup(&cleaned, &introduced);
        }
    }

    // Ruff's cleanup gives the same result for the same source and rules, which withdrawal candidates often share.
    fn cleanup(&self, source: &str, introduced: &FxHashSet<&'static str>) -> String {
        let mut rules: Vec<&'static str> = introduced.iter().copied().collect();
        rules.sort_unstable();
        // Ruff checked the source before it asked for a cleanup, so interning it finds the copy already held.
        let key = (self.sources.intern(source), rules);
        if let Some(known) = self.cleanups.lock().expect("no worker panics while cleaning up").get(&key) {
            return known.to_string();
        }
        let parser_version = self.linter.settings.from_ruff.parser_version;
        let cleaned = cleanup(self.path, source, introduced, parser_version, |source| self.check(source));
        self.cleanups.lock().expect("no worker panics while cleaning up").insert(key, self.sources.intern(&cleaned));
        cleaned
    }

    // Ruff's `lint_fix` loop: apply fixes and lint again until nothing applies, giving up when a fix breaks the syntax
    // or the passes do not settle. Withdrawn fixes keep their reports but not their fixes. The rules skip a file that
    // does not parse, so only a file that parses gets fixes, and a syntax error after one is the fix's doing.
    fn fix_with(&self, linted: &Linted, withdrawn: &mut Withdrawn) -> Result<FixerResult> {
        let mut transformed = linted.source.to_owned();
        let mut diagnostics = linted.lint.diagnostics.clone();
        let mut fixed = FixTable::default();
        let mut iterations = 0;
        let mut original = None;
        while let Some((FixResult { code, fixes }, applied)) =
            fix_file(&transformed, &diagnostics, self.unsafe_fixes, withdrawn)
        {
            errors::ensure_converging(self.path, iterations, &diagnostics)?;
            // Counted once a pass applies, from the source before the first one: the target version may reject syntax
            // the source already uses, which the fixes must not be blamed for.
            let counts = original.get_or_insert_with(|| version_counts(&linted.lint.unsupported));
            add_fixes(&mut fixed, &fixes);
            if !withdrawn.fixes.is_empty() {
                let applied_keys: Vec<FixKey> = fix_keys(&transformed, &diagnostics)
                    .into_iter()
                    .filter(|(index, _)| applied.contains(index))
                    .map(|(_, key)| key)
                    .collect();
                withdrawn.follow(&applied_keys);
            }
            transformed = code;
            iterations += 1;
            let lint = self.lint(&transformed);
            let introduced = check_syntax(&lint.diagnostics, &lint.unsupported, counts);
            errors::ensure_syntax(self.path, &transformed, &fixed, introduced)?;
            diagnostics = lint.diagnostics;
        }
        Ok(FixerResult { diagnostics, transformed, fixed })
    }

    fn lint(&self, source: &str) -> Lint {
        self.lints.get_or(&self.sources, source, || self.linter.lint(self.path, source, self.noqa))
    }

    fn check(&self, source: &str) -> Arc<Checked> {
        self.checks.get_or(&self.sources, source, || {
            Arc::new(self.linter.ruff.check(self.path, source, &self.linter.settings.from_ruff))
        })
    }
}

pub fn add_fixes(total: &mut FixTable, fixes: &FixTable) {
    for (id, secondary_code, count) in fixes.iter() {
        *total.entry(id).or_default(secondary_code) += count;
    }
}

type VersionCounts = FxHashMap<UnsupportedSyntaxErrorKind, usize>;

fn version_counts(errors: &[UnsupportedSyntaxError]) -> VersionCounts {
    let mut counts = VersionCounts::default();
    for error in errors {
        *counts.entry(error.kind).or_default() += 1;
    }
    counts
}

// The message of the syntax error a fix introduced: a parse error, or a version error of a kind the source had fewer
// of before the fixes.
fn check_syntax(
    diagnostics: &[Diagnostic],
    errors: &[UnsupportedSyntaxError],
    original: &VersionCounts,
) -> Result<(), String> {
    if let Some(invalid) = diagnostics.iter().find(|diagnostic| diagnostic.is_invalid_syntax()) {
        return Err(invalid.headline_message().to_owned());
    }
    let counts = version_counts(errors);
    let exceeded = |kind: &UnsupportedSyntaxErrorKind| counts[kind] > original.get(kind).copied().unwrap_or_default();
    match errors.iter().find(|error| exceeded(&error.kind)) {
        Some(error) => Err(error.to_string()),
        None => Ok(()),
    }
}

// Of two withdrawal candidates in rule code order, the earlier one unless the later leaves strictly less, and the
// earlier failure; as an associative choice it lets the parallel candidates reduce in any grouping to what the
// sequential loop picks, dropping each loser as soon as it is beaten.
fn earlier_unless_beaten(earlier: Result<Candidate>, later: Result<Candidate>) -> Result<Candidate> {
    let (earlier, later) = (earlier?, later?);
    Ok(if (later.settled.left, later.reapplies) < (earlier.settled.left, earlier.reapplies) { later } else { earlier })
}

// A rule withdrawn whole: the fixes it withdrew, how many Ruff rules it leaves work for, whether a second run over its
// result would fix more, and its result.
struct Candidate {
    rule: Rule,
    withdrawn: Withdrawn,
    reapplies: bool,
    settled: Settled,
}

// The fixes the loop withdrew: whole rules, and single fixes by their key. A key is the rule, the message and the text
// under the report, which a fix keeps through the passes that reparse the file, where its range moves.
#[derive(Clone, Default)]
pub struct Withdrawn {
    pub rules: RuleSet,
    pub fixes: FxHashSet<FixKey>,
}

pub type FixKey = (Rule, String, String, usize);

impl Withdrawn {
    fn with_rule(&self, rule: Rule) -> Self {
        Self { rules: self.rules.union(RuleSet::from_rules([rule])), fixes: self.fixes.clone() }
    }

    // Applying a report takes it out of the source, so a held report of the same rule, message and text after it moves
    // up one occurrence. Moving the held keys with it keeps each withheld report held through a settle's passes.
    fn follow(&mut self, applied: &[FixKey]) {
        let mut taken = FxHashMap::<(Rule, &str, &str), Vec<usize>>::default();
        for (rule, message, text, occurrence) in applied {
            taken.entry((*rule, message, text)).or_default().push(*occurrence);
        }
        self.fixes = self
            .fixes
            .drain()
            .map(|(rule, message, text, occurrence)| {
                let before = taken
                    .get(&(rule, message.as_str(), text.as_str()))
                    .map_or(0, |occurrences| occurrences.iter().filter(|&&taken_at| taken_at < occurrence).count());
                (rule, message, text, occurrence - before)
            })
            .collect();
    }

    // Whether each report is held back, by its rule or by its fix's key.
    pub fn held_back(&self, source: &str, diagnostics: &[Diagnostic]) -> Vec<bool> {
        let mut held: Vec<bool> = diagnostics
            .iter()
            .map(|diagnostic| rule_of(diagnostic).is_some_and(|rule| self.rules.contains(rule)))
            .collect();
        if !self.fixes.is_empty() {
            for (index, key) in fix_keys(source, diagnostics) {
                held[index] |= self.fixes.contains(&key);
            }
        }
        held
    }
}

// The index and key of each fixable simplint report, in source order. Reports with the same rule, message and text
// are told apart by their order, so a fix withdrawn for one of them does not withhold an identical one that is valid.
fn fix_keys(source: &str, diagnostics: &[Diagnostic]) -> Vec<(usize, FixKey)> {
    let mut reports: Vec<(usize, Rule)> = diagnostics
        .iter()
        .enumerate()
        .filter(|(_, diagnostic)| diagnostic.fix().is_some())
        .filter_map(|(index, diagnostic)| Some((index, rule_of(diagnostic)?)))
        .collect();
    reports.sort_by_key(|&(index, _)| diagnostics[index].range().map(TextRange::start));
    let mut occurrences = FxHashMap::<(Rule, String, String), usize>::default();
    reports
        .into_iter()
        .map(|(index, rule)| {
            let diagnostic = &diagnostics[index];
            let range = diagnostic.range().expect("a simplint report has a range");
            let message = diagnostic.headline_message().to_owned();
            let text = source[range].to_owned();
            let seen = occurrences.entry((rule, message.clone(), text.clone())).or_default();
            let key = (rule, message, text, *seen);
            *seen += 1;
            (index, key)
        })
        .collect()
}

// A withdrawal's state: the fixes still withheld, and the settled source they leave.
struct Admission {
    withheld: Withdrawn,
    settled: Settled,
}

// A source settled with some fixes withheld: its result, how many Ruff rules still find more in it, and the withheld
// fixes moved to follow the reports of that result, so a later run can take them over.
struct Settled {
    result: FixerResult,
    left: usize,
    held: Withdrawn,
}

// Results keyed by the source they were computed from. The lock is not held while computing, so parallel candidates
// do not wait on each other; two that need the same source at once both compute it.
struct Memo<T>(Mutex<FxHashMap<Arc<str>, T>>);

impl<T> Default for Memo<T> {
    fn default() -> Self {
        Self(Mutex::new(FxHashMap::default()))
    }
}

impl<T: Clone> Memo<T> {
    fn get_or(&self, sources: &Sources, source: &str, compute: impl FnOnce() -> T) -> T {
        if let Some(known) = self.entries().get(source) {
            return known.clone();
        }
        let computed = compute();
        self.entries().insert(sources.intern(source), computed.clone());
        computed
    }

    fn entries(&self) -> MutexGuard<'_, FxHashMap<Arc<str>, T>> {
        self.0.lock().expect("no worker panics while computing")
    }
}

// The sources a fix loop meets, each held once however many memos it keys.
#[derive(Default)]
struct Sources(Mutex<FxHashSet<Arc<str>>>);

impl Sources {
    fn intern(&self, source: &str) -> Arc<str> {
        let mut sources = self.0.lock().expect("no worker panics while computing");
        if let Some(known) = sources.get(source) {
            return Arc::clone(known);
        }
        let interned = Arc::<str>::from(source);
        sources.insert(Arc::clone(&interned));
        interned
    }
}

// The source the fixes start from, whose reports are the first pass of every fix loop.
struct Linted<'s> {
    source: &'s str,
    lint: Lint,
}

#[cfg(test)]
mod tests {
    use ruff_db::diagnostic::{Diagnostic, Span};
    use ruff_python_ast::{PySourceType, PythonVersion};
    use ruff_python_parser::{ParseOptions, UnsupportedSyntaxError, parse_unchecked};
    use ruff_source_file::SourceFileBuilder;
    use ruff_text_size::TextRange;
    use test_case::test_case;

    use super::{FixKey, VersionCounts, Withdrawn, check_syntax, version_counts};
    use crate::lint::registry::{Rule, RuleSet};

    fn version_errors(source: &str, version: PythonVersion) -> Vec<UnsupportedSyntaxError> {
        let options = ParseOptions::from(PySourceType::Python).with_target_version(version);
        parse_unchecked(source, options).unsupported_syntax_errors().to_vec()
    }

    const NESTED_QUOTE: &str = "print(f\"{x[\"a\"]}\")\n";

    #[test_case(
        "x = 1\n",
        NESTED_QUOTE,
        PythonVersion::PY311,
        Some("Cannot reuse outer quote character in f-strings on Python 3.11 (syntax was added in Python 3.12)") ;
        "reused_quote_before_3_12"
    )]
    #[test_case("x = 1\n", NESTED_QUOTE, PythonVersion::PY312, None ; "reused_quote_on_3_12")]
    #[test_case(NESTED_QUOTE, NESTED_QUOTE, PythonVersion::PY311, None ; "syntax_already_in_source")]
    #[test_case(
        "if x:\n    pass\n",
        "if (y := x):\n    pass\n",
        PythonVersion::PY37,
        Some("Cannot use named assignment expression (`:=`) on Python 3.7 (syntax was added in Python 3.8)") ;
        "walrus_before_3_8"
    )]
    fn check_syntax_version_errors(original: &str, fixed: &str, version: PythonVersion, error: Option<&str>) {
        let counts: VersionCounts = version_counts(&version_errors(original, version));
        let result = check_syntax(&[], &version_errors(fixed, version), &counts);
        assert_eq!(result.err().as_deref(), error);
    }

    #[test]
    fn check_syntax_reports_a_parse_error_first() {
        let file = SourceFileBuilder::new("module.py", "").finish();
        let diagnostic = Diagnostic::invalid_syntax(Span::from(file), "Expected an expression", TextRange::default());
        let result = check_syntax(&[diagnostic], &[], &VersionCounts::default());
        assert_eq!(result.err().as_deref(), Some("Expected an expression"));
    }

    fn key(occurrence: usize) -> FixKey {
        (Rule::SingleUseVariable, "message".to_owned(), "lock = build()".to_owned(), occurrence)
    }

    #[test_case(&[3], &[1], &[2] ; "held_moves_up_past_an_applied_identical_report")]
    #[test_case(&[3], &[0, 2], &[1] ; "held_moves_up_past_each_applied_identical_report_before_it")]
    #[test_case(&[1], &[3], &[1] ; "applied_identical_report_after_it_leaves_it")]
    fn follow_moves_held_keys_past_applied_identical_reports(held: &[usize], applied: &[usize], moved: &[usize]) {
        let mut withdrawn =
            Withdrawn { rules: RuleSet::default(), fixes: held.iter().map(|&occurrence| key(occurrence)).collect() };
        withdrawn.follow(&applied.iter().map(|&occurrence| key(occurrence)).collect::<Vec<_>>());
        let mut occurrences: Vec<usize> = withdrawn.fixes.iter().map(|fix| fix.3).collect();
        occurrences.sort_unstable();
        assert_eq!(occurrences, moved);
    }
}
