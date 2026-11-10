use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::relative_path;

// The places a report names in full; a run repeated in more modules names the first few and counts the rest.
const NAMED: usize = 3;

pub fn duplicate_code(checker: &Checker) {
    let shortest = checker.settings.project.knobs.min_duplicate_statements;
    for run in checker.project().duplicate_runs().iter().filter(|run| run.statements() >= shortest) {
        let mut places: Vec<String> =
            run.others().iter().take(NAMED).map(|(path, line)| format!("`{}:{line}`", relative_path(path))).collect();
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
