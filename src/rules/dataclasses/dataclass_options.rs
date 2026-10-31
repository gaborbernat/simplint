use ruff_diagnostics::Applicability;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::dataclasses::require_options;

pub fn dataclass_options(checker: &Checker) {
    require_options(checker, Rule::DataclassOptions, &["frozen", "slots"], |checker, class, explicit| {
        if explicit || !class.bases().is_empty() || checker.project().is_base_class(class.name.as_str()) {
            return Applicability::Unsafe;
        }
        let mut hazards = SlotHazards { found: false };
        hazards.visit_body(&class.body);
        let mutated = class.body.iter().any(|stmt| {
            stmt.as_ann_assign_stmt()
                .and_then(|field| field.target.as_name_expr())
                .is_some_and(|field| checker.project().is_attribute_assigned(field.id.as_str()))
        });
        if hazards.found || mutated { Applicability::Unsafe } else { Applicability::Safe }
    });
}

// Each of these either fails at class creation or at runtime once the class is frozen and slotted.
struct SlotHazards {
    found: bool,
}

impl SlotHazards {
    const NAMES: [&str; 7] =
        ["__slots__", "__setattr__", "__delattr__", "__dict__", "__weakref__", "cached_property", "super"];
}

impl<'a> Visitor<'a> for SlotHazards {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if stmt.as_function_def_stmt().is_some_and(|function| Self::NAMES.contains(&function.name.as_str())) {
            self.found = true;
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.found |= match expr {
            Expr::Attribute(ast::ExprAttribute { ctx: ExprContext::Store | ExprContext::Del, .. }) => true,
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => Self::NAMES.contains(&attr.as_str()),
            Expr::Name(ast::ExprName { id, .. }) => Self::NAMES.contains(&id.as_str()),
            _ => false,
        };
        visitor::walk_expr(self, expr);
    }
}
