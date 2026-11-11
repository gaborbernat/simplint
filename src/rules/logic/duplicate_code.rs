use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::relative_path;

pub fn duplicate_code(checker: &Checker) {
    let shortest = checker.settings.project.knobs.min_duplicate_statements;
    for run in checker.project().duplicate_runs().iter().filter(|run| run.statements() >= shortest) {
        // Naming every module would bury the report, so the first three are named and the rest counted.
        let mut places: Vec<String> =
            run.others().iter().take(3).map(|(path, line)| format!("`{}:{line}`", relative_path(path))).collect();
        let more = run.sharers() - places.len();
        if more > 0 {
            places.push(format!("and {more} more"));
        }
        checker.report(
            Rule::DuplicateCode,
            run.range(),
            format!(
                "These statements repeat in {}; move them into one definition the modules share",
                places.join(", ")
            ),
        );
    }
}
