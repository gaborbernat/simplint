use super::directive::{Comment, Scope};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn blanket_ignore(checker: &Checker) {
    for range in checker.comment_ranges() {
        for directive in Comment::parse(checker.text(range)).directives.iter().filter(|d| d.missing_codes) {
            let scope = match directive.scope {
                Scope::Line => "on the line",
                Scope::File => "in the file",
            };
            let message = format!(
                "`# {}` suppresses every error {scope}; name the codes, as in `# {}`",
                directive.name,
                directive.example()
            );
            checker.report(Rule::BlanketIgnore, range, message);
        }
    }
}
