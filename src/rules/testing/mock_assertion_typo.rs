use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::syntax::last_segment;

// The assertion methods of `unittest.mock`; a name is a typo of one of them, not of a mock attribute.
const ASSERTIONS: &[&str] = &[
    "assert_any_call",
    "assert_called",
    "assert_called_once",
    "assert_called_once_with",
    "assert_called_with",
    "assert_has_calls",
    "assert_not_called",
    "assert_any_await",
    "assert_awaited",
    "assert_awaited_once",
    "assert_awaited_once_with",
    "assert_awaited_with",
    "assert_has_awaits",
    "assert_not_awaited",
];

// The prefixes `unittest.mock` rejects on a mock without `unsafe=True`, from Python 3.10.
const GUARDED_PREFIXES: &[&str] = &["assert", "assret", "asert", "aseert", "assrt"];

// Two edits cover a dropped or swapped letter in a prefix and a missing or extra letter; more would match
// unrelated names.
const MAX_DISTANCE: usize = 2;

pub fn mock_assertion_typo(checker: &Checker) {
    let mut finder = Typos { found: Vec::new() };
    finder.visit_body(checker.body());
    for (attr, intended) in finder.found {
        // A name the file or the project defines is an attribute of some class, not a mock method.
        let name = attr.as_str();
        if checker.project().is_attribute_assigned(name)
            || checker.function_defs().iter().any(|function| function.name.as_str() == name)
        {
            continue;
        }
        let fix = Fix::unsafe_edit(Edit::range_replacement(intended.to_owned(), attr.range()));
        checker.report_fix(
            Rule::MockAssertionTypo,
            attr.range(),
            format!("`{name}` is not a mock assertion; it is `{intended}`"),
            Some(fix),
            format!("Rename to `{intended}`"),
        );
    }
}

struct Typos<'a> {
    found: Vec<(&'a ast::Identifier, &'static str)>,
}

impl<'a> Visitor<'a> for Typos<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            // Code under `assertRaises` or `pytest.raises` expects the call to fail, so the misspelling is the point.
            Stmt::With(ast::StmtWith { items, .. }) if items.iter().any(|item| expects_error(&item.context_expr)) => {}
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Attribute(attribute) = expr
            && attribute.ctx.is_load()
            && let Some(intended) = intended_assertion(attribute.attr.as_str())
        {
            self.found.push((&attribute.attr, intended));
        }
        visitor::walk_expr(self, expr);
    }
}

fn expects_error(expr: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return false;
    };
    last_segment(func).is_some_and(|name| name == "raises" || name.starts_with("assertRaises"))
}

// The assertion a name is meant to be: the same name with the `assert_` prefix, or the nearest assertion when the
// name is within reach of exactly one. `called` stays out, since every mock has that attribute.
fn intended_assertion(name: &str) -> Option<&'static str> {
    if name == "called" || ASSERTIONS.contains(&name) {
        return None;
    }
    if let Some(&intended) = ASSERTIONS.iter().find(|intended| intended.strip_prefix("assert_") == Some(name)) {
        return Some(intended);
    }
    // A misspelled prefix still starts with the letters of `as`, which keeps the distances off the other names. A
    // mock raises `AttributeError` for a name with one of the prefixes it guards, so that test fails without the
    // lint, while other classes define such names, as `assert_failed` shows.
    if !name.starts_with("as") || GUARDED_PREFIXES.iter().any(|prefix| name.starts_with(prefix)) {
        return None;
    }
    // Names differ by at least as many edits as their lengths do, so an assertion further off than that is out of reach
    // and cannot tie with one in reach either; most names, such as numpy's `astype`, are measured against none.
    let length = name.chars().count();
    let mut candidates: Vec<(usize, &'static str)> = ASSERTIONS
        .iter()
        .filter(|intended| intended.len().abs_diff(length) <= MAX_DISTANCE)
        .map(|&intended| (edit_distance(name, intended), intended))
        .collect();
    candidates.sort_unstable();
    let &(distance, intended) = candidates.first()?;
    let runner_up = candidates.get(1).map_or(usize::MAX, |&(distance, _)| distance);
    (distance <= MAX_DISTANCE && distance < runner_up).then_some(intended)
}

fn edit_distance(from: &str, to: &str) -> usize {
    let to: Vec<char> = to.chars().collect();
    let mut previous: Vec<usize> = (0..=to.len()).collect();
    for (row, from_char) in from.chars().enumerate() {
        let mut current = vec![row + 1];
        for (column, to_char) in to.iter().enumerate() {
            let substitution = previous[column] + usize::from(from_char != *to_char);
            current.push(substitution.min(previous[column + 1] + 1).min(current[column] + 1));
        }
        previous = current;
    }
    previous[to.len()]
}
