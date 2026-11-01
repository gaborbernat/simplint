use std::collections::HashSet;

use ruff_diagnostics::Applicability;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::receivers::module_name;
use crate::rules::dataclasses::{Offer, require_options};

pub fn dataclass_options(checker: &Checker) {
    require_options(checker, Rule::DataclassOptions, &["frozen", "slots"], |checker, decorated| {
        let class = decorated.class;
        let name = class.name.as_str();
        let qualified = format!("{}.{name}", module_name(checker.module_path(), checker.path));
        let project = checker.project();
        let fields: HashSet<&str> = class
            .body
            .iter()
            .filter_map(|stmt| stmt.as_ann_assign_stmt()?.target.as_name_expr().map(|field| field.id.as_str()))
            .collect();
        let mut hazards = Hazards { fields: &fields, frozen: false, slots: false, suspected: false, depth: 0 };
        hazards.visit_body(&class.body);
        for attribute in project.attributes_written_on(&qualified) {
            hazards.written(attribute);
        }
        hazards.slots |= project.has_cached_property_subclass(&qualified);
        hazards.suspected |= decorated.explicit
            || !class.bases().is_empty()
            || project.is_base_class(name)
            || project.has_namesake_hazards(&qualified)
            || fields.iter().any(|field| project.is_attribute_written(field));
        let options: Vec<&'static str> = decorated
            .missing
            .iter()
            .copied()
            .filter(|option| match *option {
                "frozen" => !hazards.frozen,
                _ => !hazards.slots,
            })
            .collect();
        let applicability = if hazards.suspected { Applicability::Unsafe } else { Applicability::Safe };
        Some(Offer { options, edits: Vec::new(), applicability })
    });
}

// What breaks once the class is frozen or slotted. A hazard the class shows for certain withholds the option it
// breaks; one that only might break it, such as an attribute with a field's name set on an object of unknown class,
// makes the fix unsafe.
struct Hazards<'f> {
    fields: &'f HashSet<&'f str>,
    frozen: bool,
    slots: bool,
    suspected: bool,
    // How many functions deep the walk is, so a write to `self` is known to come from a method.
    depth: usize,
}

impl Hazards<'_> {
    const SUSPECTED: [&str; 5] = ["__dict__", "__weakref__", "__setattr__", "__delattr__", "super"];

    // A frozen instance rejects every write, and a slotted one has no room for an attribute that is not a field.
    fn written(&mut self, attribute: &str) {
        self.frozen = true;
        self.slots |= !self.fields.contains(attribute);
    }

    fn mention(&mut self, name: &str) {
        if name == "cached_property" {
            self.slots = true;
        }
        self.suspected |= Self::SUSPECTED.contains(&name);
    }
}

impl<'a> Visitor<'a> for Hazards<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            // `frozen=True` refuses to overwrite these, and `slots=True` refuses a class that declares its own slots.
            Stmt::FunctionDef(function) if matches!(function.name.as_str(), "__setattr__" | "__delattr__") => {
                self.frozen = true;
            }
            Stmt::FunctionDef(_) => {
                self.depth += 1;
                visitor::walk_stmt(self, stmt);
                self.depth -= 1;
                return;
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                if self.depth > 0 && value.as_name_expr().is_some_and(|name| name.id.as_str() == "self") {
                    self.written(attr.as_str());
                } else {
                    self.suspected = true;
                }
            }
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, .. }) if id.as_str() == "__slots__" => {
                self.slots = true;
            }
            Expr::Name(ast::ExprName { id, .. }) => self.mention(id.as_str()),
            Expr::Attribute(ast::ExprAttribute { attr, .. }) => self.mention(attr.as_str()),
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}
