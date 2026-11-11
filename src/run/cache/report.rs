use std::fs;
use std::path::Path;
use std::sync::OnceLock;

use ruff_db::diagnostic::{
    Annotation, Diagnostic, DiagnosticId, LintName, SecondaryCode, Severity, Span, SubDiagnosticSeverity, UnifiedFile,
};
use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::{IOError, Violation};
use ruff_source_file::{SourceFile, SourceFileBuilder};
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::FxHashMap;

use crate::fix::{FixKey, Withdrawn};
use crate::lint::checker::source_file_at;
use crate::lint::registry::{Rule, RuleSet};
use crate::run::Diagnostics;

// The reports a file had without fixes, under the fingerprint they were reported under.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct Findings {
    pub(super) fingerprint: u64,
    pub(super) reports: Vec<Report>,
}

impl Findings {
    // The `full` text is kept beside the entry, so a run that shows other output never reads it.
    pub(super) fn diagnostics(archived: &ArchivedFindings, path: &Path) -> Option<Diagnostics> {
        let reports: Vec<Report> = rkyv::deserialize::<_, rkyv::rancor::Error>(&archived.reports).ok()?;
        // A file that cannot be read has no source to read back, and its report is built from none.
        let source = if reports.iter().all(|report| matches!(report.kind, ReportKind::IoError)) {
            String::new()
        } else {
            fs::read_to_string(path).ok()?
        };
        let file = SourceFileBuilder::new(path.to_string_lossy(), source.as_str()).finish();
        let inner = reports.iter().map(|report| report.diagnostic(&file)).collect::<Option<_>>()?;
        Some(Diagnostics { inner, ..Diagnostics::default() })
    }
}

// The rule of a code, looked up in a map since a warm run looks up every cached report.
fn rule_by_code(code: &str) -> Option<Rule> {
    static RULES: OnceLock<FxHashMap<&'static str, Rule>> = OnceLock::new();
    RULES.get_or_init(|| Rule::ALL.iter().map(|rule| (rule.code(), *rule)).collect()).get(code).copied()
}

// A diagnostic a rule reported, or one of the two a file gets that no rule reports, with the fields the output shows.
// `Report::of` takes only a diagnostic the checker builds, so `Report::diagnostic` gives back the same one.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct Report {
    kind: ReportKind,
    message: String,
    start: u32,
    end: u32,
    help: Vec<String>,
    fix: Option<StoredFix>,
    // The other locations the report points at, in the order the checker gave them.
    related: Vec<StoredRelated>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum ReportKind {
    // The rule's code, and the line of its suppression comment, as `Suppressions::locate` sets it.
    Rule { code: String, noqa_offset: u32 },
    // A file that cannot be read, which `Diagnostics::from_error` builds from its message alone.
    IoError,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct StoredRelated {
    path: String,
    start: u32,
    end: u32,
    message: String,
}

impl StoredRelated {
    // The location a secondary annotation names, or `None` when it is not one the checker set: a file its source
    // cannot be read back from, or one without a message.
    fn of(annotation: &Annotation) -> Option<Self> {
        let span = annotation.get_span();
        let UnifiedFile::Ruff(file) = span.file() else { return None };
        let range = span.range()?;
        Some(Self {
            path: file.name().to_owned(),
            start: range.start().to_u32(),
            end: range.end().to_u32(),
            message: annotation.get_message()?.to_owned(),
        })
    }
}

impl Report {
    // All of the reports, or `None` when a diagnostic is one `of` cannot store. Only a rule's diagnostic has a
    // suppression line, so an unreadable file takes none.
    pub(super) fn all(diagnostics: &[Diagnostic], noqa_rows: &[TextSize]) -> Option<Vec<Self>> {
        let mut rows = noqa_rows.iter().copied();
        diagnostics.iter().map(|diagnostic| Self::of(diagnostic, rows.next())).collect()
    }

    // The report of a diagnostic. Only what the checker sets is stored: a rule's diagnostic with more than its one
    // primary annotation, a parent, a severity other than error, or a sub-diagnostic other than a help note is not.
    fn of(diagnostic: &Diagnostic, noqa_row: Option<TextSize>) -> Option<Self> {
        let primary = diagnostic.annotations().first()?;
        let range = primary.get_span().range()?;
        let unruled = |kind| {
            Some(Self {
                kind,
                message: diagnostic.headline_message().to_owned(),
                start: range.start().to_u32(),
                end: range.end().to_u32(),
                help: Vec::new(),
                fix: None,
                related: Vec::new(),
            })
        };
        if diagnostic.id() == DiagnosticId::Lint(RuffRule::IOError.name()) {
            return unruled(ReportKind::IoError);
        }
        let rule = rule_by_code(diagnostic.secondary_code()?.as_str())?;
        let related: Vec<StoredRelated> =
            diagnostic.secondary_annotations().map(StoredRelated::of).collect::<Option<_>>()?;
        let help: Vec<String> = diagnostic
            .sub_diagnostics()
            .iter()
            .map(|sub| {
                let help = sub.severity() == SubDiagnosticSeverity::Help && sub.annotations().is_empty();
                help.then(|| sub.headline_message().to_owned())
            })
            .collect::<Option<_>>()?;
        let plain = diagnostic.annotations().iter().filter(|annotation| annotation.is_primary()).count() == 1
            && primary.is_primary()
            && primary.get_message().is_none()
            && diagnostic.primary_tags().is_none_or(<[_]>::is_empty)
            && diagnostic.id() == DiagnosticId::Lint(LintName::of(rule.name()))
            && diagnostic.severity() == Severity::Error
            && diagnostic.documentation_url() == Some(rule.url().as_str())
            && diagnostic.parent().is_none();
        let fix = diagnostic.fix().map(StoredFix::of);
        let noqa_offset = noqa_row?.to_u32();
        plain.then(|| Self {
            kind: ReportKind::Rule { code: rule.code().to_owned(), noqa_offset },
            message: diagnostic.headline_message().to_owned(),
            start: range.start().to_u32(),
            end: range.end().to_u32(),
            help,
            fix,
            related,
        })
    }

    // The diagnostic `of` took the report from, given the file it reports on, or `None` when a range does not fit the
    // file as it now reads.
    fn diagnostic(&self, file: &SourceFile) -> Option<Diagnostic> {
        let length = file.source_text().len();
        let range = stored_range(self.start, self.end, length)?;
        let (rule, noqa_offset) = match &self.kind {
            ReportKind::IoError => {
                return Some(IOError { message: self.message.clone() }.into_diagnostic(range, file));
            }
            ReportKind::Rule { code, noqa_offset } => (rule_by_code(code)?, *noqa_offset),
        };
        stored_range(noqa_offset, noqa_offset, length)?;
        let mut diagnostic =
            Diagnostic::new(DiagnosticId::Lint(LintName::of(rule.name())), Severity::Error, self.message.as_str());
        diagnostic.annotate(Annotation::primary(Span::from(file.clone()).with_range(range)));
        for related in &self.related {
            let file = source_file_at(Path::new(&related.path))?;
            let range = stored_range(related.start, related.end, file.source_text().len())?;
            diagnostic
                .annotate(Annotation::secondary(Span::from(file).with_range(range)).message(related.message.as_str()));
        }
        diagnostic.set_secondary_code(SecondaryCode::new(rule.code().to_owned()));
        diagnostic.set_documentation_url(Some(rule.url()));
        for help in &self.help {
            diagnostic.help(help.as_str());
        }
        if let Some(fix) = &self.fix {
            diagnostic.set_fix(fix.fix(length)?);
        }
        diagnostic.set_noqa_offset(TextSize::new(noqa_offset));
        Some(diagnostic)
    }
}

// A fix as rkyv data. Ruff's `Fix` keeps its fields private, so `fix` rebuilds it from its edits and levels, in the
// order they were stored, which is the order `Fix` sorts them into.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct StoredFix {
    applicability: StoredApplicability,
    isolation: StoredIsolation,
    edits: Vec<StoredEdit>,
}

impl StoredFix {
    fn of(fix: &Fix) -> Self {
        Self {
            applicability: match fix.applicability() {
                Applicability::DisplayOnly => StoredApplicability::DisplayOnly,
                Applicability::Unsafe => StoredApplicability::Unsafe,
                Applicability::Safe => StoredApplicability::Safe,
            },
            isolation: match fix.isolation() {
                IsolationLevel::Group(group) => StoredIsolation::Group(group),
                IsolationLevel::NonOverlapping => StoredIsolation::NonOverlapping,
            },
            edits: fix
                .edits()
                .iter()
                .map(|edit| StoredEdit {
                    start: edit.start().to_u32(),
                    end: edit.end().to_u32(),
                    content: edit.content().map(str::to_owned),
                })
                .collect(),
        }
    }

    // `None` for a fix without edits, which no rule builds and which a cache file therefore cannot hold, or with an edit
    // outside a source of `length` bytes.
    fn fix(&self, length: usize) -> Option<Fix> {
        let applicability = match self.applicability {
            StoredApplicability::DisplayOnly => Applicability::DisplayOnly,
            StoredApplicability::Unsafe => Applicability::Unsafe,
            StoredApplicability::Safe => Applicability::Safe,
        };
        let isolation = match self.isolation {
            StoredIsolation::Group(group) => IsolationLevel::Group(group),
            StoredIsolation::NonOverlapping => IsolationLevel::NonOverlapping,
        };
        let edits: Vec<Edit> = self.edits.iter().map(|edit| edit.edit(length)).collect::<Option<_>>()?;
        let (first, rest) = edits.split_first()?;
        Some(Fix::applicable_edits(first.clone(), rest.iter().cloned(), applicability).isolate(isolation))
    }
}

#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum StoredApplicability {
    DisplayOnly,
    Unsafe,
    Safe,
}

#[derive(Clone, Copy, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
enum StoredIsolation {
    Group(u32),
    NonOverlapping,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct StoredEdit {
    start: u32,
    end: u32,
    content: Option<String>,
}

impl StoredEdit {
    fn edit(&self, length: usize) -> Option<Edit> {
        let range = stored_range(self.start, self.end, length)?;
        Some(match &self.content {
            Some(content) => Edit::range_replacement(content.clone(), range),
            None => Edit::range_deletion(range),
        })
    }
}

// A range read back from a cache file, or `None` when it cannot lie in a source of `length` bytes, as in a cache file
// that was damaged or edited by hand, or a file changed without changing its key.
fn stored_range(start: u32, end: u32, length: usize) -> Option<TextRange> {
    (start <= end && usize::try_from(end).is_ok_and(|end| end <= length))
        .then(|| TextRange::new(TextSize::new(start), TextSize::new(end)))
}

// The fix loop's last run over an unchanged file: the fingerprint it ran under and the fixes it withdrew, rules by code
// and single fixes by rule code, message and text. Codes, unlike a rule's position in the rule set, stay put while
// development builds of a version add rules.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct Settled {
    pub(super) fingerprint: u64,
    rules: Vec<String>,
    fixes: Vec<WithdrawnFix>,
}

#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
struct WithdrawnFix {
    rule: String,
    message: String,
    text: String,
    occurrence: usize,
}

impl Settled {
    pub(super) fn new(fingerprint: u64, withdrawn: &Withdrawn) -> Self {
        Self {
            fingerprint,
            rules: withdrawn.rules.iter().map(|rule| rule.code().to_owned()).collect(),
            fixes: withdrawn
                .fixes
                .iter()
                .map(|key| WithdrawnFix {
                    rule: key.rule.code().to_owned(),
                    message: key.message.clone(),
                    text: key.text.clone(),
                    occurrence: key.occurrence,
                })
                .collect(),
        }
    }
}

pub(super) fn withdrawn(settled: &ArchivedSettled) -> Withdrawn {
    Withdrawn {
        rules: RuleSet::from_rules(settled.rules.iter().filter_map(|code| Rule::from_code(code.as_str()))),
        fixes: settled
            .fixes
            .iter()
            .filter_map(|fix| {
                Some(FixKey {
                    rule: Rule::from_code(fix.rule.as_str())?,
                    message: fix.message.as_str().to_owned(),
                    text: fix.text.as_str().to_owned(),
                    occurrence: fix.occurrence.to_native() as usize,
                })
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use ruff_diagnostics::{Applicability, Edit, Fix, IsolationLevel};
    use ruff_text_size::{TextRange, TextSize};
    use test_case::test_case;

    use ruff_source_file::SourceFileBuilder;

    use super::{Report, ReportKind, StoredApplicability, StoredEdit, StoredFix, StoredIsolation};

    const SOURCE_LENGTH: usize = 30;

    fn range(start: u32, end: u32) -> TextRange {
        TextRange::new(TextSize::new(start), TextSize::new(end))
    }

    // The fix a cache file holds after an rkyv round trip, which is what a later run reads back.
    fn round_trip(fix: &Fix) -> Option<Fix> {
        let bytes = rkyv::to_bytes::<rkyv::rancor::Error>(&StoredFix::of(fix)).expect("serialize");
        rkyv::from_bytes::<StoredFix, rkyv::rancor::Error>(&bytes).expect("deserialize").fix(SOURCE_LENGTH)
    }

    #[test_case(Fix::safe_edit(Edit::range_replacement("y".to_owned(), range(4, 9))) ; "safe_replacement")]
    #[test_case(
        Fix::unsafe_edits(
            Edit::range_deletion(range(0, 3)),
            [Edit::insertion("import os\n".to_owned(), TextSize::new(12)), Edit::range_replacement("z".to_owned(), range(20, 25))],
        )
        .isolate(IsolationLevel::Group(7)) ; "unsafe_edits_in_a_group"
    )]
    #[test_case(Fix::display_only_edit(Edit::deletion(TextSize::new(2), TextSize::new(2))) ; "display_only_deletion")]
    fn fix_survives_a_round_trip(fix: Fix) {
        assert_eq!(round_trip(&fix), Some(fix));
    }

    #[test]
    fn fix_without_edits_is_not_restored() {
        let stored = StoredFix {
            applicability: StoredApplicability::Safe,
            isolation: StoredIsolation::NonOverlapping,
            edits: Vec::new(),
        };
        assert!(stored.fix(SOURCE_LENGTH).is_none());
    }

    #[test_case(0, 1, 0, true ; "fits")]
    #[test_case(1, 0, 0, false ; "end_before_start")]
    #[test_case(0, 12, 0, false ; "end_past_the_source")]
    #[test_case(0, 1, 12, false ; "suppression_past_the_source")]
    fn report_is_restored_only_inside_the_source(start: u32, end: u32, noqa_offset: u32, restored: bool) {
        let file = SourceFileBuilder::new("module.py", "x = 1\ny = 2").finish();
        let report = Report {
            kind: ReportKind::Rule { code: "SMP101".to_owned(), noqa_offset },
            message: "Single-letter name".to_owned(),
            start,
            end,
            help: Vec::new(),
            fix: None,
            related: Vec::new(),
        };
        assert_eq!(report.diagnostic(&file).is_some(), restored);
    }

    #[test_case(5, 3 ; "end_before_start")]
    #[test_case(3, 31 ; "end_past_the_source")]
    fn fix_with_an_edit_outside_the_source_is_not_restored(start: u32, end: u32) {
        let stored = StoredFix {
            applicability: StoredApplicability::Safe,
            isolation: StoredIsolation::NonOverlapping,
            edits: vec![StoredEdit { start, end, content: None }],
        };
        assert!(stored.fix(SOURCE_LENGTH).is_none());
    }

    #[test]
    fn applicability_and_isolation_are_kept() {
        let fix = Fix::applicable_edit(Edit::deletion(TextSize::new(0), TextSize::new(1)), Applicability::Unsafe)
            .isolate(IsolationLevel::NonOverlapping);
        assert_eq!(
            round_trip(&fix).map(|fix| (fix.applicability(), fix.isolation())),
            Some((Applicability::Unsafe, IsolationLevel::NonOverlapping))
        );
    }
}
