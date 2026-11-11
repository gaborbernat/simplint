use ruff_python_ast::helpers::Truthiness;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use rustc_hash::{FxHashMap, FxHashSet};

use super::visitor::FactVisitor;
use crate::syntax::{alias_root_binding, is_main_test, is_type_checking_name, pattern_capture};

impl FactVisitor<'_> {
    pub(super) fn branches(&self, test: &Expr) -> (Runs, Runs) {
        let name = test.as_name_expr().map(|name| name.id.as_str());
        let typing_guard =
            is_type_checking_name(test) && !name.is_some_and(|name| self.constants.truthy.contains(name));
        if typing_guard || is_main_test(test) || name.is_some_and(|name| self.constants.falsy.contains(name)) {
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

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Runs {
    Always,
    Below(u8, u8),
    Never,
}

impl Runs {
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

// The full `sys.version_info` is longer than the tuple it is compared with, so it never equals that tuple.
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
    // A major-only check that holds on Python 3, the only major a supported Python has, leaves its else branch
    // unreachable.
    let three_holds = match op {
        ast::CmpOp::Eq => bound.0 == 3,
        ast::CmpOp::LtE => 3 <= bound.0,
        ast::CmpOp::Lt => 3 < bound.0,
        _ => false,
    };
    let other = if inclusive && three_holds { Runs::Never } else { Runs::Always };
    Some(if holds_below { (gate, other) } else { (Runs::Always, gate) })
}

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

#[derive(Default)]
pub(super) struct Constants<'a> {
    pub(super) falsy: FxHashSet<&'a str>,
    pub(super) truthy: FxHashSet<&'a str>,
}

pub(super) fn constants(body: &[Stmt]) -> Constants<'_> {
    let mut bindings = Bindings::default();
    bindings.visit_body(body);
    let mut globals = GlobalNames::default();
    globals.visit_body(body);
    // A `global` declaration rebinds a name while the module runs, and a star import binds the public names, so
    // neither leaves a name a constant.
    for name in globals.names {
        bindings.bind(name, None);
    }
    let mut constants = Constants::default();
    let star = bindings.star;
    for (name, constant) in bindings.bound {
        if star && !name.starts_with('_') {
            continue;
        }
        match constant {
            Some(true) => constants.truthy.insert(name),
            Some(false) => constants.falsy.insert(name),
            None => false,
        };
    }
    constants
}

#[derive(Default)]
struct Bindings<'a> {
    bound: FxHashMap<&'a str, Option<bool>>,
    star: bool,
}

impl<'a> Bindings<'a> {
    fn bind(&mut self, name: &'a str, constant: Option<bool>) {
        self.bound.entry(name).and_modify(|once| *once = None).or_insert(constant);
    }

    fn bind_target(&mut self, target: &'a Expr, constant: Option<bool>) {
        match target.as_name_expr() {
            Some(name) => self.bind(name.id.as_str(), constant),
            None => self.visit_expr(target),
        }
    }
}

impl<'a> Visitor<'a> for Bindings<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                let constant = literal_constant(value);
                for target in targets {
                    self.bind_target(target, constant);
                }
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) => {
                self.bind_target(target, value.as_deref().and_then(literal_constant));
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            // A function or a class binds its own name only; what its body binds is local to it.
            Stmt::FunctionDef(function) => self.bind(function.name.as_str(), None),
            Stmt::ClassDef(class) => self.bind(class.name.as_str(), None),
            Stmt::Import(ast::StmtImport { names, .. }) => {
                for alias in names {
                    self.bind(alias_root_binding(alias), None);
                }
            }
            Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.star |= names.iter().any(|alias| alias.name.as_str() == "*");
                for alias in names.iter().filter(|alias| alias.name.as_str() != "*") {
                    self.bind(alias_root_binding(alias), None);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, .. }) = expr {
            self.bind(id.as_str(), None);
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name, .. }) = handler;
        if let Some(name) = name {
            self.bind(name.as_str(), None);
        }
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a ast::Pattern) {
        if let Some(name) = pattern_capture(pattern) {
            self.bind(name.as_str(), None);
        }
        visitor::walk_pattern(self, pattern);
    }
}

#[derive(Default)]
struct GlobalNames<'a> {
    names: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for GlobalNames<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Global(ast::StmtGlobal { names, .. }) = stmt {
            self.names.extend(names.iter().map(ast::Identifier::as_str));
        }
        visitor::walk_stmt(self, stmt);
    }
}

fn literal_constant(value: &Expr) -> Option<bool> {
    value
        .is_literal_expr()
        .then(|| matches!(Truthiness::from_expr(value, |_| false), Truthiness::True | Truthiness::Truthy))
}
