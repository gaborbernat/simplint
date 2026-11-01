use ruff_diagnostics::{Edit, Fix};
use ruff_python_trivia::is_pragma_comment;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstrings::PublicApi;
use crate::rules::{docstring, statement_deletion};

pub fn function_docstring(checker: &Checker) {
    let body = checker.body();
    let api = PublicApi::of(checker, body);
    let owners = checker
        .function_defs()
        .iter()
        .filter(|function| !api.covers(function.range))
        .map(|function| (function.body.as_slice(), "Function"))
        .chain(
            checker
                .class_defs()
                .iter()
                .filter(|class| !api.covers(class.range))
                .map(|class| (class.body.as_slice(), "Class")),
        )
        .filter(|(owner_body, _)| docstring(owner_body).is_some());
    for (owner_body, kind) in owners {
        let stmt = &owner_body[0];
        // `ruff format` puts a body on lines of its own, so the fix must not depend on whether it already is. A pragma
        // after the docstring, such as `# noqa: E501`, covers its line only and goes with it.
        let edit = if owner_body.len() == 1 {
            let pragma = checker.trailing_comment(stmt).filter(|comment| is_pragma_comment(checker.text(*comment)));
            Some(Edit::range_replacement("...".to_owned(), stmt.range().cover(pragma.unwrap_or(stmt.range()))))
        } else {
            statement_deletion(checker, stmt.range(), owner_body.get(1).map(Ranged::range), true)
                .map(Edit::range_deletion)
        };
        let fix = edit.map(Fix::unsafe_edit);
        checker.report_fix(
            Rule::FunctionDocstring,
            stmt.range(),
            format!("{kind} docstring on internal code; let the name, signature and types document it"),
            fix,
            "Remove the docstring",
        );
    }
}
