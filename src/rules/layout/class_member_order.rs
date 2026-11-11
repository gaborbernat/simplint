use ruff_python_ast::helpers::any_over_body;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use super::class_order::{Arrangement, ClassOrder, Inversion, Section};
use super::reorder::{Suite, moved_span, moves_definition_unsafely, reorder_fix};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::semantic::leaves_django_order_to_ruff;

pub fn class_member_order(checker: &Checker) {
    let orders = checker.state().class_orders(checker);
    for class in checker.class_defs() {
        check_class(checker, class, &orders[&class.range]);
    }
}

fn check_class(checker: &Checker, class: &ast::StmtClassDef, order: &ClassOrder<'_>) {
    let inversions = order.inversions();
    if inversions.is_empty() {
        return;
    }
    let expected = order.expected(Arrangement::Sections);
    let mut position = vec![0; expected.len()];
    for (place, &index) in expected.iter().enumerate() {
        position[index] = place;
    }
    let suite = Suite { body: &class.body, blank_lines: 1 };
    let mut pending =
        moved_span(&expected).and_then(|span| reorder_fix(checker, &suite, &expected, span, shifts_unsafely));
    // The layout resolves an inversion when it puts the member above its anchor. Otherwise the member is pinned behind
    // the anchor, and its report has no fix.
    let resolved = |inversion: &Inversion| position[inversion.statement] < position[inversion.anchor];
    // Ruff's DJ012 reports a statement of a Django model that is out of the style guide order, without a fix, so the
    // layout leaves that report to Ruff unless it is the one that carries the class's fix.
    let dj012 = leaves_django_order_to_ruff(checker.settings);
    let left_to_ruff =
        |inversion: &Inversion| dj012 && checker.semantic().reported_by_dj012(class.body[inversion.statement].range());
    let carrier = pending.as_ref().and_then(|_| {
        let mut candidates = inversions.iter().enumerate().filter(|(_, inversion)| resolved(inversion));
        let first = candidates.clone().next();
        candidates.find(|(_, inversion)| !left_to_ruff(inversion)).or(first).map(|(index, _)| index)
    });
    for (index, inversion) in inversions.iter().enumerate() {
        let carries_fix = carrier == Some(index);
        if left_to_ruff(inversion) && !carries_fix {
            continue;
        }
        let message = match inversion.anchor_section {
            Some(Section::Constructor(_)) if matches!(inversion.section, Section::Constructor(_)) => {
                format!(
                    "`{}` belongs above `{}`, in the order Python runs the constructors",
                    inversion.name, inversion.anchor_name
                )
            }
            _ => format!(
                "`{}` belongs with {}, above `{}`",
                inversion.name,
                inversion.section.noun(),
                inversion.anchor_name
            ),
        };
        let fix = if carries_fix { pending.take() } else { None };
        checker.report_fix(
            Rule::ClassMemberOrder,
            inversion.range,
            message,
            fix,
            format!("Move `{}` above `{}`", inversion.name, inversion.anchor_name),
        );
    }
}

// A statement that is not a definition moves safely when it calls nothing: a call can observe the order, as `next()` on
// a shared counter or `locals()` does.
fn shifts_unsafely(stmt: &Stmt) -> bool {
    moves_definition_unsafely(stmt).unwrap_or_else(|| any_over_body(std::slice::from_ref(stmt), Expr::is_call_expr))
}
