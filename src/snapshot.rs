use std::fmt::Write;
use std::path::{Path, PathBuf};

use ruff_diagnostics::Applicability;
use ruff_python_ast::PythonVersion;
use ruff_source_file::LineIndex;

use crate::linter::Linter;
use crate::project::Project;
use crate::registry::{Rule, RuleSet};
use crate::settings::Settings;

pub fn check(rule: Rule, fixture: &str, target_version: PythonVersion) {
    let suffix =
        if target_version == PythonVersion::PY314 { String::new() } else { format!("_py3{}", target_version.minor) };
    insta::assert_snapshot!(
        format!("{}_{}{suffix}", rule.code(), fixture.replace('/', "_")),
        render(rule, fixture, target_version)
    );
}

fn render(rule: Rule, fixture: &str, target_version: PythonVersion) -> String {
    let fixtures = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/test/fixtures");
    let path = fixtures.join(fixture);
    let source = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let mut settings = Settings::new(fixtures.clone());
    settings.target_version = target_version;
    settings.dependencies.insert("requests".to_owned());
    let files: Vec<PathBuf> = ignore::WalkBuilder::new(&fixtures)
        .build()
        .filter_map(Result::ok)
        .map(ignore::DirEntry::into_path)
        .filter(|candidate| candidate.extension().is_some_and(|ext| ext == "py"))
        .collect();
    let project = Project::index(&files, &["SMP113_scripts:run".to_owned()]);
    let linter = Linter { settings: &settings, project: &project, rules: RuleSet::from_rules([rule]) };
    let diagnostics = linter.lint(&path, &source).expect("fixture parses");
    let index = LineIndex::from_source_text(&source);
    let mut output = String::new();
    for diagnostic in &diagnostics {
        let location = index.line_column(diagnostic.range.start(), &source);
        let fix = match &diagnostic.fix {
            Some(fix) if fix.applies(Applicability::Safe) => " [fix]",
            Some(_) => " [unsafe fix]",
            None => "",
        };
        writeln!(
            output,
            "{}:{} {} {}{fix}",
            location.line,
            location.column,
            diagnostic.rule.code(),
            diagnostic.message
        )
        .expect("writing to a string");
    }
    if diagnostics.iter().any(|diagnostic| diagnostic.fix.is_some()) {
        let outcome = linter.fix(&path, &source, Applicability::Unsafe).expect("fixes apply");
        write!(
            output,
            "--- fixed ({} applied, {} remaining) ---\n{}",
            outcome.fixed,
            outcome.diagnostics.len(),
            outcome.source
        )
        .expect("writing to a string");
    }
    output
}
