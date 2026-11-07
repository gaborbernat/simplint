use rustc_hash::FxHashSet;
use std::cell::OnceCell;

use ruff_diagnostics::Applicability;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};

use crate::lint::checker::Checker;
use crate::project::receivers::module_name;
use crate::rules::classes::Decorated;
use crate::syntax::{is_name, last_segment, receiver};

// What freezing or slotting the class would break, and whether the walk met code it cannot judge, which makes the fix
// unsafe.
pub(super) struct Analysis {
    pub(super) frozen_breaks: bool,
    pub(super) slots_breaks: bool,
    suspected: bool,
}

impl Analysis {
    pub(super) fn applicability(&self) -> Applicability {
        if self.suspected { Applicability::Unsafe } else { Applicability::Safe }
    }
}

// `None` when no option of the dataclass rules gets a fix: a class without fields, such as a fixture standing for a
// dataclass with the default options, has no state the options would protect, and a test expecting the class to fail
// keeps it as is.
pub(super) fn analyze<'a>(
    checker: &Checker<'a>,
    decorated: &Decorated,
    expected_to_raise: &OnceCell<FxHashSet<&'a str>>,
) -> Option<Analysis> {
    let class = decorated.class;
    let name = class.name.as_str();
    let qualified = format!("{}.{name}", module_name(checker.module_path(), checker.path));
    let project = checker.project();
    if !class.body.iter().any(Stmt::is_ann_assign_stmt)
        || expected_to_raise.get_or_init(|| expected_to_raise_classes(checker.body())).contains(name)
    {
        return None;
    }
    let (class_variables, fields): (Vec<_>, Vec<_>) = class
        .body
        .iter()
        .filter_map(|stmt| {
            let stmt = stmt.as_ann_assign_stmt()?;
            Some((stmt.target.as_name_expr()?.id.as_str(), is_class_variable(&stmt.annotation)))
        })
        .partition(|(_, class_variable)| *class_variable);
    let fields: FxHashSet<&str> = fields.into_iter().map(|(field, _)| field).collect();
    let mut hazards =
        Hazards { fields: &fields, frozen: false, slots: false, suspected: false, depth: 0, receiver: None };
    hazards.visit_body(&class.body);
    for attribute in project.attributes_written_on(&qualified) {
        hazards.written(attribute);
    }
    // A write through a receiver simplint cannot type, such as `envs[name].active = True`, may target this class. A
    // base's methods may write to the instance, and a frozen dataclass can neither inherit from nor be inherited by
    // a mutable one.
    hazards.frozen |= fields.iter().any(|field| project.is_attribute_written(field))
        || class.bases().iter().any(|base| !is_generic(base))
        || project.is_base_class(name);
    // A class variable written through an instance becomes read-only once slots leave no `__dict__` to shadow it.
    if class_variables.iter().any(|(variable, _)| project.is_attribute_written(variable)) {
        hazards.frozen = true;
        hazards.slots = true;
    }
    hazards.slots |= project.has_unslottable_subclass(&qualified);
    // Only a frozen slotted dataclass gets a `__getstate__`: without one, pickle protocols 0 and 1 refuse a slotted
    // instance, and with one, a pickled subclass instance keeps only the fields. The fixes of both options apply
    // together, so the state is the one after freezing, whichever fix goes first.
    let frozen = sets_true(decorated, "frozen") || !hazards.frozen;
    hazards.slots |= if frozen { project.has_stateful_subclass(&qualified) } else { project.pickles_low_protocol() };
    let explicit = decorated.keywords.iter().any(|keyword| {
        keyword.arg.as_ref().is_some_and(|arg| matches!(arg.as_str(), "frozen" | "slots")) && !is_true(&keyword.value)
    });
    hazards.suspected |= explicit || !class.bases().is_empty() || project.has_namesake_hazards(&qualified);
    Some(Analysis { frozen_breaks: hazards.frozen, slots_breaks: hazards.slots, suspected: hazards.suspected })
}

// The names the module calls inside `pytest.raises(...)` or `self.assertRaises(...)`: a test creating an instance there
// expects it to fail somewhere, as an unhashable dict key, and either option can change that.
fn expected_to_raise_classes(body: &[Stmt]) -> FxHashSet<&str> {
    let mut blocks = Raises { depth: 0, called: FxHashSet::default() };
    blocks.visit_body(body);
    blocks.called
}

struct Raises<'a> {
    // How many `with` blocks expecting an exception the walk is in.
    depth: usize,
    called: FxHashSet<&'a str>,
}

impl<'a> Visitor<'a> for Raises<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let Stmt::With(with) = stmt else {
            return visitor::walk_stmt(self, stmt);
        };
        let raises = with.items.iter().any(|item| {
            item.context_expr.as_call_expr().is_some_and(|call| {
                last_segment(&call.func).is_some_and(|name| matches!(name, "raises" | "assertRaises"))
            })
        });
        self.depth += usize::from(raises);
        visitor::walk_stmt(self, stmt);
        self.depth -= usize::from(raises);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.depth > 0
            && let Some(func) = expr.as_call_expr().and_then(|call| call.func.as_name_expr())
        {
            self.called.insert(func.id.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}

// `ClassVar[...]` and `InitVar[...]` annotations declare no instance field, and so no slot.
fn is_class_variable(annotation: &Expr) -> bool {
    if let Some(text) = annotation.as_string_literal_expr() {
        let text = text.value.to_str();
        return text.contains("ClassVar") || text.contains("InitVar");
    }
    let annotation = annotation.as_subscript_expr().map_or(annotation, |subscript| &subscript.value);
    last_segment(annotation).is_some_and(|name| matches!(name, "ClassVar" | "InitVar"))
}

fn is_generic(base: &Expr) -> bool {
    let base = base.as_subscript_expr().map_or(base, |subscript| &subscript.value);
    last_segment(base) == Some("Generic")
}

fn sets_true(decorated: &Decorated, option: &str) -> bool {
    decorated
        .keywords
        .iter()
        .any(|keyword| keyword.arg.as_ref().is_some_and(|arg| arg.as_str() == option) && is_true(&keyword.value))
        || decorated.splat.iter().any(|(name, value)| *name == option && is_true(value))
}

fn is_true(value: &Expr) -> bool {
    value.as_boolean_literal_expr().is_some_and(|literal| literal.value)
}

// What breaks once the class is frozen or slotted. A hazard the class shows for certain withholds the option it
// breaks; one that only might break it, such as a class body writing to something other than `self`, makes the fix
// unsafe.
struct Hazards<'f, 'a> {
    fields: &'f FxHashSet<&'f str>,
    frozen: bool,
    slots: bool,
    suspected: bool,
    // How many functions deep the walk is, so a write to the receiver is known to come from a method.
    depth: usize,
    // The receiver of the method being walked, which a nested def keeps: a write through it is an instance write.
    receiver: Option<&'a str>,
}

impl Hazards<'_, '_> {
    const SUSPECTED: [&str; 5] = ["__dict__", "__weakref__", "__setattr__", "__delattr__", "super"];

    // A frozen instance rejects every write, and a slotted one has no room for an attribute that is not a field. Writing
    // into `__dict__` gets past `frozen=True`, but a slotted instance has none.
    fn written(&mut self, attribute: &str) {
        self.frozen |= attribute != "__dict__";
        self.slots |= !self.fields.contains(attribute);
    }

    fn mention(&mut self, name: &str) {
        if name == "cached_property" {
            self.slots = true;
        }
        self.suspected |= Self::SUSPECTED.contains(&name);
    }
}

impl<'a> Visitor<'a> for Hazards<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            // `frozen=True` refuses to overwrite these, and `slots=True` refuses a class that declares its own slots.
            Stmt::FunctionDef(function) if matches!(function.name.as_str(), "__setattr__" | "__delattr__") => {
                self.frozen = true;
            }
            Stmt::FunctionDef(function) => {
                let outer = self.receiver;
                if self.depth == 0 {
                    self.receiver = receiver(function);
                }
                self.depth += 1;
                visitor::walk_stmt(self, stmt);
                self.depth -= 1;
                self.receiver = outer;
                return;
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, attr, ctx: ExprContext::Store | ExprContext::Del, .. }) => {
                if self.depth > 0 && self.receiver.is_some_and(|receiver| is_name(value, receiver)) {
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
