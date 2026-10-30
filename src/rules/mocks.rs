use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{expr_mentions, for_each_body, is_alone_on_lines};

const MOCK_CLASSES: &[&str] = &["Mock", "MagicMock", "AsyncMock", "NonCallableMock", "NonCallableMagicMock"];

pub fn mock_without_spec(checker: &mut Checker) {
    let mut finder = Unspecced { found: Vec::new() };
    finder.visit_body(checker.body());
    for (range, call) in finder.found {
        let message = match call {
            MockCall::Mock => "Mock without a spec; build it with `create_autospec` or pass `spec=`",
            MockCall::Patch | MockCall::PatchObject => "Patch without a spec; pass `autospec=True`",
        };
        checker.report(Rule::MockWithoutSpec, range, message, None);
    }
}

struct Unspecced {
    found: Vec<(TextRange, MockCall)>,
}

enum MockCall {
    Mock,
    Patch,
    PatchObject,
}

impl<'a> Visitor<'a> for Unspecced {
    fn visit_expr(&mut self, expr: &'a Expr) {
        const SPEC_KEYWORDS: &[&str] = &["spec", "spec_set", "wraps", "autospec", "new", "new_callable"];
        if let Expr::Call(call) = expr
            && let Some(kind) = classify(&call.func)
        {
            let positional_spec = match kind {
                MockCall::Mock => 1,
                MockCall::Patch => 2,
                MockCall::PatchObject => 3,
            };
            let specced = call.arguments.args.len() >= positional_spec
                || call
                    .arguments
                    .keywords
                    .iter()
                    .any(|keyword| keyword.arg.as_ref().is_none_or(|arg| SPEC_KEYWORDS.contains(&arg.as_str())));
            if !specced {
                self.found.push((call.range(), kind));
            }
        }
        visitor::walk_expr(self, expr);
    }
}

fn classify(func: &Expr) -> Option<MockCall> {
    let name = UnqualifiedName::from_expr(func)?;
    let segments = name.segments();
    match segments.last()? {
        last if MOCK_CLASSES.contains(last) => Some(MockCall::Mock),
        &"patch" => Some(MockCall::Patch),
        &"object" if segments.len() >= 2 && segments[segments.len() - 2] == "patch" => Some(MockCall::PatchObject),
        _ => None,
    }
}

pub fn mock_attribute_assignment(checker: &mut Checker) {
    let mut found = Vec::new();
    for_each_body(checker.body(), &mut |statements, _| {
        for (index, stmt) in statements.iter().enumerate() {
            let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
                continue;
            };
            let ([Expr::Name(target)], Expr::Call(call)) = (targets.as_slice(), value.as_ref()) else {
                continue;
            };
            if !UnqualifiedName::from_expr(&call.func).is_some_and(|name| {
                name.segments().last().is_some_and(|last| MOCK_CLASSES.contains(last) || *last == "create_autospec")
            }) {
                continue;
            }
            let assignments = following_assignments(target.id.as_str(), &statements[index + 1..]);
            if !assignments.is_empty() {
                found.push((call, assignments));
            }
        }
    });
    let source = checker.source;
    for (call, assignments) in found {
        let fixable = assignments.iter().all(|assignment| is_alone_on_lines(source, assignment.stmt.range()))
            && !checker.comments().any(|comment| {
                assignments
                    .iter()
                    .any(|assignment| source.full_lines_range(assignment.stmt.range()).contains_range(comment))
            });
        let fix = fixable.then(|| {
            let close = call.arguments.end() - TextSize::from(1);
            let separator = if call.arguments.is_empty() {
                ""
            } else if checker.text(TextRange::new(call.arguments.start(), close)).trim_end().ends_with(',') {
                " "
            } else {
                ", "
            };
            let keywords: Vec<String> = assignments
                .iter()
                .map(|assignment| format!("{}={}", assignment.attr, checker.text(assignment.value.range())))
                .collect();
            let deletions = assignments
                .iter()
                .map(|assignment| Edit::range_deletion(source.full_lines_range(assignment.stmt.range())));
            Fix::safe_edits(Edit::insertion(format!("{separator}{}", keywords.join(", ")), close), deletions)
        });
        let names: Vec<&str> = assignments.iter().map(|assignment| assignment.attr).collect();
        checker.report(
            Rule::MockAttributeAssignment,
            assignments[0].stmt.range(),
            format!("Pass {} through the mock constructor instead of assigning after creation", names.join(", ")),
            fix,
        );
    }
}

struct Assignment<'a> {
    stmt: &'a Stmt,
    attr: &'a str,
    value: &'a Expr,
}

fn following_assignments<'a>(mock: &str, statements: &'a [Stmt]) -> Vec<Assignment<'a>> {
    // These constructor arguments configure the mock itself, so passing one differs from setting the attribute.
    const RESERVED_KEYWORDS: &[&str] =
        &["spec", "spec_set", "wraps", "name", "parent", "unsafe", "instance", "_name", "_parent"];
    let mut assignments: Vec<Assignment> = Vec::new();
    for stmt in statements {
        let Stmt::Assign(ast::StmtAssign { targets, value, .. }) = stmt else {
            break;
        };
        let [Expr::Attribute(ast::ExprAttribute { value: owner, attr, .. })] = targets.as_slice() else {
            break;
        };
        let attr = attr.as_str();
        if owner.as_name_expr().is_none_or(|name| name.id.as_str() != mock)
            || RESERVED_KEYWORDS.contains(&attr)
            || assignments.iter().any(|existing| existing.attr == attr)
            || expr_mentions(value, mock)
        {
            break;
        }
        assignments.push(Assignment { stmt, attr, value });
    }
    assignments
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::MockAttributeAssignment, "tests/test_SMP203.py" ; "mock_attribute_assignment")]
    #[test_case(Rule::MockWithoutSpec, "tests/test_SMP204.py" ; "mock_without_spec")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
