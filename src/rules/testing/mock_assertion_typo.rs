use ruff_diagnostics::{Applicability, Edit};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::fix_of;
use crate::rules::testing::recognize::expects_error;

pub fn mock_assertion_typo(checker: &Checker) {
    let mut finder = Typos { found: Vec::new(), fixable: FxHashMap::default(), ruff_reported: FxHashSet::default() };
    finder.visit_body(checker.body());
    let defined: FxHashSet<&str> = checker.function_defs().iter().map(|function| function.name.as_str()).collect();
    for (attr, intended) in finder.found {
        // A name the file or the project defines is an attribute of some class, not a mock method.
        let name = attr.as_str();
        if checker.project().is_attribute_assigned(name) || defined.contains(name) {
            continue;
        }
        // A comment in the text the fix drops would be lost with it.
        let fix = finder
            .fixable
            .get(&attr.range())
            .filter(|deletions| !deletions.iter().any(|&range| checker.has_comment_in(range)))
            .and_then(|deletions| {
                let rename = Edit::range_replacement(intended.to_owned(), attr.range());
                let mut edits: Vec<Edit> = deletions.iter().map(|range| Edit::range_deletion(*range)).collect();
                edits.push(rename);
                fix_of(edits, Applicability::Unsafe)
            });
        if fix.is_none() && finder.ruff_reported.contains(&attr.range()) {
            continue;
        }
        checker.report_fix(
            Rule::MockAssertionTypo,
            attr.range(),
            format!("`{name}` is not a mock assertion; it is `{intended}`"),
            fix,
            format!("Rename to `{intended}`"),
        );
    }
}

struct Typos<'a> {
    found: Vec<(&'a ast::Identifier, &'static str)>,
    // The names called as a statement of their own, by their range, which the rename turns into a check. A call that is
    // the whole test of an `assert` maps to the ranges to drop, the `assert ` prefix and the parentheses around the
    // test, since an assertion method returns `None`. Renamed anywhere else, the name would be a bound method, which is
    // always true, or a `None` the code goes on to use.
    fixable: FxHashMap<TextRange, Vec<TextRange>>,
    // The names Ruff's PGH005 reports, by their range, which this rule leaves to it when it has no fix for them.
    ruff_reported: FxHashSet<TextRange>,
}

impl<'a> Visitor<'a> for Typos<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Expr(ast::StmtExpr { value, .. }) => {
                if let Some(attr) = called_attribute(value) {
                    self.fixable.insert(attr.range(), Vec::new());
                }
            }
            Stmt::Assert(ast::StmtAssert { test, msg: None, .. }) => {
                if let Some(attr) = called_attribute(test) {
                    let mut deletions = vec![TextRange::new(stmt.start(), test.start())];
                    // The statement's range includes the closing parenthesis of a parenthesized test.
                    if stmt.end() > test.end() {
                        deletions.push(TextRange::new(test.end(), stmt.end()));
                    }
                    self.fixable.insert(attr.range(), deletions);
                }
            }
            _ => {}
        }
        match stmt {
            // Code under `assertRaises` or `pytest.raises` expects the call to fail, so the misspelling is the point.
            Stmt::With(ast::StmtWith { items, .. }) if items.iter().any(|item| expects_error(&item.context_expr)) => {}
            Stmt::Assert(ast::StmtAssert { test: expr, .. }) | Stmt::Expr(ast::StmtExpr { value: expr, .. }) => {
                self.ruff_reported.extend(ruff_reported_name(expr).map(Ranged::range));
                visitor::walk_stmt(self, stmt);
            }
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

// The name of the attribute that Ruff's PGH005 reports in the test of an `assert` or in a statement of its own: the
// expression itself or the callee of a call. The operand of a `not` or `and`, or a comparison, is not checked by Ruff,
// so it stays with this rule.
fn ruff_reported_name(expr: &Expr) -> Option<&ast::Identifier> {
    // The names Ruff's PGH005 reports as non-existent mock methods, bare or called.
    const RUFF_NON_EXISTENT_NAMES: &[&str] = &[
        "any_call",
        "called_once",
        "called_once_with",
        "called_with",
        "has_calls",
        "not_called",
        "any_await",
        "awaited",
        "awaited_once",
        "awaited_once_with",
        "awaited_with",
        "has_awaits",
        "not_awaited",
    ];
    let attribute = match expr {
        Expr::Attribute(attribute) => attribute,
        Expr::Call(ast::ExprCall { func, .. }) => match func.as_ref() {
            Expr::Attribute(attribute) => attribute,
            _ => return None,
        },
        _ => return None,
    };
    RUFF_NON_EXISTENT_NAMES.contains(&attribute.attr.as_str()).then_some(&attribute.attr)
}

fn called_attribute(expr: &Expr) -> Option<&ast::Identifier> {
    let Expr::Call(ast::ExprCall { func, .. }) = expr else {
        return None;
    };
    func.as_attribute_expr().map(|attribute| &attribute.attr)
}

// The assertion a name is meant to be: the same name with the `assert_` prefix, or the nearest assertion when the
// name is within reach of exactly one. `called` stays out, since every mock has that attribute.
fn intended_assertion(name: &str) -> Option<&'static str> {
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
