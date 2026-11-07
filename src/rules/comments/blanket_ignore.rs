use ruff_linter::registry::Rule as RuffRule;

use super::directive::Scope;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn blanket_ignore(checker: &Checker) {
    for (range, comment) in checker.state().comments(checker) {
        for directive in comment.directives.iter().filter(|d| d.missing_codes && !is_left_to_ruff(checker, d.ruff_rule))
        {
            let scope = match directive.scope {
                Scope::Line => "on the line",
                Scope::File | Scope::Module => "in the file",
            };
            let message = format!(
                "`# {}` suppresses every error {scope}; name the codes, as in `# {}`",
                directive.name,
                directive.example()
            );
            checker.report(Rule::BlanketIgnore, *range, message);
        }
    }
}

// Ruff's PGH003 and PGH004 report the blanket forms they cover, and RUF103 the `ruff: ignore` forms without codes, so
// those are left to Ruff while it runs them.
fn is_left_to_ruff(checker: &Checker, ruff_rule: Option<RuffRule>) -> bool {
    ruff_rule.is_some_and(|rule| checker.settings.from_ruff.ruff_runs(rule))
}
