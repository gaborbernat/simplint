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
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_parser::{ParseOptions, UnsupportedSyntaxError, UnsupportedSyntaxErrorKind, parse_unchecked};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::fix::apply::{FixResult, fix_file};
use crate::fix::cleanup::cleanup;
use crate::lint::Linter;
use crate::lint::registry::{Rule, RuleSet, rule_of};
use crate::lint::ruff::Checked;

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
    lints: Memo<Vec<Diagnostic>>,
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
    // original file did not have gets Ruff's own fix when Ruff has one, and otherwise the fixes of the simplint rule that
    // caused it are withdrawn, keeping that rule's reports. `ruff format`, which runs last, keeps the syntax tree, so it
    // cannot undo this.
    //
    // A run withdraws a rule whose fixes leave Ruff work next to another rule's pending fixes, yet once those apply, a
    // run over the result can offer the withdrawn fixes again. Running until a run changes nothing, or returns to a
    // source seen before, makes a single run final.
    pub fn fix(&self, source: &str) -> Result<(FixerResult, RuleSet)> {
        let mut fixed = FixTable::default();
        let mut seen = FxHashSet::from_iter([source.to_owned()]);
        let mut current = source.to_owned();
        let (mut result, unfixable) = loop {
            let (result, unfixable) = self.run(&current)?;
            add_fixes(&mut fixed, &result.fixed);
            if result.fixed.is_empty() || !seen.insert(result.transformed.clone()) {
                break (result, unfixable);
            }
            errors::ensure_converging(self.path, seen.len(), &result.diagnostics)?;
            current.clone_from(&result.transformed);
        };
        result.fixed = fixed;
        Self::withdraw(&mut result.diagnostics, unfixable);
        Ok((result, unfixable))
    }

    // A withdrawn fix is no longer offered, so the summary does not count it as fixable.
    pub fn withdraw(diagnostics: &mut [Diagnostic], rules: RuleSet) {
        for diagnostic in diagnostics {
            if rule_of(diagnostic).is_some_and(|rule| rules.contains(rule)) {
                diagnostic.remove_fix();
            }
        }
    }

    fn run(&self, source: &str) -> Result<(FixerResult, RuleSet)> {
        let linted = Linted { source, diagnostics: self.lint(source) };
        let mut unfixable = RuleSet::default();
        let (mut result, mut left) = self.settle(&linted, unfixable)?;
        while left > 0 {
            // Rules are tried in code order, so the same file always yields the same fixes. Among withdrawals that leave
            // as little for Ruff, one whose result a second run leaves alone wins, so a single run is final. The
            // candidates run in parallel and are compared in that order, which keeps the choice the same.
            let applied: BTreeSet<Rule> = Self::fixing_rules(&result.fixed).collect();
            let fewest = applied
                .par_iter()
                .map(|&rule| {
                    let unfixable = unfixable.union(RuleSet::from_rules([rule]));
                    let (result, left) = self.settle(&linted, unfixable)?;
                    let reapplies = left == 0 && self.reapplies(&result.transformed)?;
                    Ok(Candidate { unfixable, left, reapplies, result })
                })
                .reduce_with(earlier_unless_beaten)
                .transpose()?;
            // Every fix comes from a simplint rule, so a fix that left Ruff work has a rule to withdraw.
            Candidate { unfixable, left, result, .. } = fewest.expect("an applied fix names its rule");
        }
        // Fixes Ruff's cleanup undoes, such as a relative import the project bans, leave the source as it was, and a
        // fix that changes nothing is not offered.
        if result.transformed == source && !result.fixed.is_empty() {
            unfixable = unfixable.union(RuleSet::from_rules(Self::fixing_rules(&result.fixed)));
            result = FixerResult {
                diagnostics: linted.diagnostics.clone(),
                transformed: source.to_owned(),
                fixed: FixTable::default(),
            };
        }
        Ok((result, unfixable))
    }

    fn fixing_rules(fixes: &FixTable) -> impl Iterator<Item = Rule> {
        fixes.iter().filter_map(|(_, code, _)| Rule::from_code(code?.as_str()))
    }

    // Whether another run over the fixed source would apply fixes and leave Ruff nothing new.
    fn reapplies(&self, source: &str) -> Result<bool> {
        let (result, left) = self.settle(&Linted { source, diagnostics: self.lint(source) }, RuleSet::default())?;
        Ok(!result.fixed.is_empty() && result.transformed != source && left == 0)
    }

    // The fixed result and how many Ruff rules still find more in it than in the original. Ruff's cleanup can give the
    // rules new work, as dropping a branch the target version always takes can leave a constant one function reads, so
    // fixes and cleanups alternate until the fixes bring Ruff nothing new, or the cleanup returns to code it already
    // gave, as when it undoes a fix.
    fn settle(&self, linted: &Linted, unfixable: RuleSet) -> Result<(FixerResult, usize)> {
        let mut seen = FxHashSet::from_iter([linted.source.to_owned()]);
        let mut fixed = FixTable::default();
        let mut round = self.fix_with(linted, unfixable)?;
        loop {
            if round.fixed.is_empty() {
                return Ok((FixerResult { fixed, ..round }, 0));
            }
            add_fixes(&mut fixed, &round.fixed);
            let Some((transformed, left)) = self.clean_up(linted.source, &round.transformed) else {
                return Ok((FixerResult { fixed, ..round }, 0));
            };
            let diagnostics = self.lint(&transformed);
            if left > 0 || !seen.insert(transformed.clone()) {
                return Ok((FixerResult { diagnostics, transformed, fixed }, left));
            }
            errors::ensure_converging(self.path, seen.len(), &diagnostics)?;
            round = self.fix_with(&Linted { source: &transformed, diagnostics }, unfixable)?;
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
    // or the passes do not settle. Rules in `unfixable` keep their reports but not their fixes. The rules skip a file
    // that does not parse, so only a file that parses gets fixes, and a syntax error after one is the fix's doing.
    fn fix_with(&self, linted: &Linted, unfixable: RuleSet) -> Result<FixerResult> {
        let version = self.linter.settings.from_ruff.target_version;
        let mut transformed = linted.source.to_owned();
        let mut diagnostics = linted.diagnostics.clone();
        let mut fixed = FixTable::default();
        let mut iterations = 0;
        let mut original = None;
        while let Some(FixResult { code, fixes }) = fix_file(&transformed, &diagnostics, self.unsafe_fixes, unfixable) {
            errors::ensure_converging(self.path, iterations, &diagnostics)?;
            // Counted once a pass applies, from the source before the first one: the target version may reject syntax
            // the source already uses, which the fixes must not be blamed for.
            let counts = original.get_or_insert_with(|| version_counts(&version_errors(&transformed, version)));
            add_fixes(&mut fixed, &fixes);
            transformed = code;
            iterations += 1;
            diagnostics = self.lint(&transformed);
            let introduced = check_syntax(&diagnostics, &version_errors(&transformed, version), counts);
            errors::ensure_syntax(self.path, &transformed, &fixed, introduced)?;
        }
        Ok(FixerResult { diagnostics, transformed, fixed })
    }

    fn lint(&self, source: &str) -> Vec<Diagnostic> {
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

// The errors of the syntax a version rejects, such as an f-string reusing its quote before Python 3.12.
fn version_errors(source: &str, version: PythonVersion) -> Vec<UnsupportedSyntaxError> {
    let options = ParseOptions::from(PySourceType::Python).with_target_version(version);
    parse_unchecked(source, options).unsupported_syntax_errors().to_vec()
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
    Ok(if (later.left, later.reapplies) < (earlier.left, earlier.reapplies) { later } else { earlier })
}

// A withdrawal: the rules without fixes, how many Ruff rules it leaves work for, whether a second run over its result
// would fix more, and its result.
struct Candidate {
    unfixable: RuleSet,
    left: usize,
    reapplies: bool,
    result: FixerResult,
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
    diagnostics: Vec<Diagnostic>,
}

#[cfg(test)]
mod tests {
    use ruff_db::diagnostic::{Diagnostic, Span};
    use ruff_python_ast::PythonVersion;
    use ruff_source_file::SourceFileBuilder;
    use ruff_text_size::TextRange;
    use test_case::test_case;

    use super::{VersionCounts, check_syntax, version_counts, version_errors};

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
}
