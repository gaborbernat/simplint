use ruff_python_ast::helpers::Truthiness;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::{FxHashMap, FxHashSet};

use super::visitor::FactVisitor;
use crate::syntax::is_main_test;

impl FactVisitor<'_> {
    // When the body of an `if` with this test runs, and when the clauses after it can.
    pub(super) fn branches(&self, test: &Expr) -> (Runs, Runs) {
        let falsy_name = test.as_name_expr().is_some_and(|name| self.falsy_constants.contains(name.id.as_str()));
        if is_type_checking(test) || is_main_test(test) || falsy_name {
            return (Runs::Never, Runs::Always);
        }
        if let Some(branches) = version_branches(test) {
            return branches;
        }
        match Truthiness::from_expr(test, |_| false) {
            Truthiness::False | Truthiness::Falsey | Truthiness::None => (Runs::Never, Runs::Always),
            Truthiness::True | Truthiness::Truthy => (Runs::Always, Runs::Never),
            Truthiness::Unknown => (Runs::Always, Runs::Always),
        }
    }
}

// When a statement runs as the module loads: always, only on Pythons older than a `(major, minor)` version, or never.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Runs {
    Always,
    Below(u8, u8),
    Never,
}

impl Runs {
    // When a statement runs that needs both conditions to hold.
    pub(super) fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Never, _) | (_, Self::Never) => Self::Never,
            (Self::Below(major, minor), Self::Below(other_major, other_minor)) => {
                let (major, minor) = (major, minor).min((other_major, other_minor));
                Self::Below(major, minor)
            }
            (Self::Below(..), Self::Always) => self,
            (Self::Always, _) => other,
        }
    }
}

// Whether an `if` test is `TYPE_CHECKING`, bare or as `typing.TYPE_CHECKING`, which is false at run time.
fn is_type_checking(test: &Expr) -> bool {
    match test {
        Expr::Name(name) => name.id.as_str() == "TYPE_CHECKING",
        Expr::Attribute(attribute) => attribute.attr.as_str() == "TYPE_CHECKING",
        _ => false,
    }
}

// When the branches of a `sys.version_info` check run: `sys.version_info < (3, 8)` holds only below 3.8, and
// `sys.version_info >= (3, 8)` fails only below it. The full `sys.version_info` is longer than the tuple it is compared
// with, so it never equals it; its first item, as `sys.version_info[0]` or `.major`, compares as a number.
fn version_branches(test: &Expr) -> Option<(Runs, Runs)> {
    let ast::ExprCompare { ops, operands, .. } = test.as_compare_expr()?;
    let ([op], [left, right]) = (&**ops, &**operands) else {
        return None;
    };
    let number = |expr: &Expr| expr.as_number_literal_expr()?.value.as_int()?.as_u8();
    let is_version_info =
        |expr: &Expr| UnqualifiedName::from_expr(expr).is_some_and(|name| name.segments() == ["sys", "version_info"]);
    let (bound, inclusive) = if is_version_info(left) {
        let ast::ExprTuple { elts, .. } = right.as_tuple_expr()?;
        let bound = match &**elts {
            [major] => (number(major)?, 0),
            [major, minor] => (number(major)?, number(minor)?),
            _ => return None,
        };
        (bound, false)
    } else {
        let major = match left {
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) if number(slice) == Some(0) => value,
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if attr.as_str() == "major" => value,
            _ => return None,
        };
        if !is_version_info(major) {
            return None;
        }
        ((number(right)?, 0), true)
    };
    // The version a check of the first item names with `<=` or `==`, as `== 2`, holds below the next major version.
    let next = (bound.0.checked_add(1)?, 0);
    let (below, holds_below) = match op {
        ast::CmpOp::LtE | ast::CmpOp::Eq if inclusive => (next, true),
        ast::CmpOp::Lt | ast::CmpOp::LtE => (bound, true),
        ast::CmpOp::Gt if inclusive => (next, false),
        ast::CmpOp::GtE | ast::CmpOp::Gt => (bound, false),
        _ => return None,
    };
    let gate = Runs::Below(below.0, below.1);
    Some(if holds_below { (gate, Runs::Always) } else { (Runs::Always, gate) })
}

// Whether an `except` clause catches `ImportError` or `ModuleNotFoundError`, alone or in a tuple.
pub(super) fn catches_import_error(type_: &Expr) -> bool {
    let is_import_error = |expr: &Expr| {
        UnqualifiedName::from_expr(expr)
            .and_then(|name| name.segments().last().copied())
            .is_some_and(|name| matches!(name, "ImportError" | "ModuleNotFoundError"))
    };
    match type_ {
        Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().any(is_import_error),
        _ => is_import_error(type_),
    }
}

// The names the module binds exactly once at its top level, to a constant that is false, as `_false = 0` does.
pub(super) fn falsy_constants(body: &[Stmt]) -> FxHashSet<&str> {
    let mut bound: FxHashMap<&str, bool> = FxHashMap::default();
    for stmt in body {
        let (targets, value): (&[Expr], Option<&Expr>) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => (targets, Some(value)),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) => {
                (std::slice::from_ref(target), value.as_deref())
            }
            Stmt::AugAssign(ast::StmtAugAssign { target, .. }) => (std::slice::from_ref(target), None),
            _ => continue,
        };
        let falsy = value.is_some_and(|value| {
            value.is_literal_expr()
                && matches!(
                    Truthiness::from_expr(value, |_| false),
                    Truthiness::False | Truthiness::Falsey | Truthiness::None
                )
        });
        for target in targets.iter().filter_map(Expr::as_name_expr) {
            bound.entry(target.id.as_str()).and_modify(|once| *once = false).or_insert(falsy);
        }
    }
    bound.into_iter().filter(|(_, falsy)| *falsy).map(|(name, _)| name).collect()
}
