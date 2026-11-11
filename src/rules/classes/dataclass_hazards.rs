use rustc_hash::FxHashSet;

use ruff_diagnostics::Applicability;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};

use crate::lint::checker::Checker;
use crate::project::receivers::module_name;
use crate::rules::classes::Decorated;
use crate::rules::testing::recognize::is_raises_or_assert_raises;
use crate::syntax::{is_classmethod, is_name, is_true_literal, last_segment, receiver};

// What freezing or slotting the class would break, and whether the walk met code it cannot judge, which makes the fix
// unsafe.
#[expect(clippy::struct_excessive_bools, reason = "each flag is one finding of the walk")]
pub(super) struct Analysis {
    pub(super) frozen_breaks: bool,
    pub(super) slots_breaks: bool,
    // Whether the walk found an instance written after construction, which `frozen` breaks for certain.
    pub(super) mutated: bool,
    // The fields the plain class's `__init__` would take, `InitVar`s included.
    pub(super) parameters: usize,
    suspected: bool,
    // Whether the class has a keyword such as `metaclass=`, which `slots=True` runs again when it builds the class.
    keyworded: bool,
}

impl Analysis {
    pub(super) fn applicability(&self) -> Applicability {
        if self.suspected { Applicability::Unsafe } else { Applicability::Safe }
    }

    pub(super) fn slots_applicability(&self) -> Applicability {
        if self.suspected || self.keyworded { Applicability::Unsafe } else { Applicability::Safe }
    }
}

// `None` when no option of the dataclass rules gets a fix: a class without fields, such as a fixture standing for a
// dataclass with the default options, has no state the options would protect, and a test expecting the class to fail
// keeps it as is.
pub(super) fn analyze(checker: &Checker<'_>, decorated: &Decorated) -> Option<Analysis> {
    let class = decorated.class;
    let name = class.name.as_str();
    if !class.body.iter().any(Stmt::is_ann_assign_stmt) || checker.state().expected_to_raise(checker).contains(name) {
        return None;
    }
    let qualified = format!("{}.{name}", module_name(checker.module_path(), checker.path));
    let fields = Fields::of(class);
    let mut hazards = Hazards {
        fields: &fields.names,
        defaults: &fields.defaults,
        frozen: false,
        slots: false,
        mutated: false,
        suspected: false,
        depth: 0,
        receiver: None,
        classmethod: false,
        nested: None,
        own: None,
    };
    hazards.visit_body(&class.body);
    index_hazards(checker, decorated, &qualified, &fields, &mut hazards);
    Some(Analysis {
        frozen_breaks: hazards.frozen,
        slots_breaks: hazards.slots,
        mutated: hazards.mutated,
        parameters: fields.parameters,
        keyworded: !class.keywords().is_empty(),
        suspected: hazards.suspected,
    })
}

// The annotated names of a class body: the fields its instances store, its class variables, the defaults of its fields,
// and the parameters its generated `__init__` takes.
struct Fields<'a> {
    names: FxHashSet<&'a str>,
    class_variables: Vec<&'a str>,
    defaults: FxHashSet<&'a str>,
    parameters: usize,
}

impl<'a> Fields<'a> {
    fn of(class: &'a ast::StmtClassDef) -> Self {
        let (class_variables, fields): (Vec<_>, Vec<_>) = class
            .body
            .iter()
            .filter_map(|stmt| {
                let stmt = stmt.as_ann_assign_stmt()?;
                Some((stmt.target.as_name_expr()?.id.as_str(), is_class_variable(&stmt.annotation)))
            })
            .partition(|(_, class_variable)| *class_variable);
        let parameters = class
            .body
            .iter()
            .filter_map(|stmt| stmt.as_ann_assign_stmt().filter(|stmt| stmt.target.is_name_expr()))
            .filter(|stmt| {
                !annotated_as(&stmt.annotation, &["ClassVar", "KW_ONLY"])
                    && field_keyword(stmt.value.as_deref(), "init")
                        .and_then(Expr::as_boolean_literal_expr)
                        .is_none_or(|init| init.value)
            })
            .count();
        let names: FxHashSet<&str> = fields.into_iter().map(|(field, _)| field).collect();
        let defaults: FxHashSet<&str> = class
            .body
            .iter()
            .filter_map(Stmt::as_ann_assign_stmt)
            .filter(|stmt| has_default(stmt.value.as_deref()))
            .filter_map(|stmt| Some(stmt.target.as_name_expr()?.id.as_str()))
            .filter(|name| names.contains(name))
            .collect();
        Self {
            names,
            class_variables: class_variables.into_iter().map(|(variable, _)| variable).collect(),
            defaults,
            parameters,
        }
    }
}

// What the project index knows about the class, which the walk of its body cannot see: writes from other files, and
// the subclasses that a frozen or slotted class would break.
fn index_hazards(checker: &Checker, decorated: &Decorated, qualified: &str, fields: &Fields, hazards: &mut Hazards) {
    let class = decorated.class;
    let name = class.name.as_str();
    let project = checker.project();
    for attribute in project.attributes_written_on(qualified) {
        // The index records `__dict__` for reads too, as `vars(obj)` or `obj.__dict__.update(...)`, and `__weakref__`
        // for weak references: a slotted instance has neither, though freezing stops neither. A store to `__dict__` is
        // recorded apart.
        match attribute {
            "__dict__" | "__weakref__" => hazards.slots = true,
            _ => hazards.written(attribute),
        }
    }
    if project.dict_stored_on(qualified) {
        hazards.written("__dict__");
    }
    // A write through a receiver simplint cannot type, such as `envs[name].active = True`, may target this class. A
    // base's methods may write to the instance, and a frozen dataclass can neither inherit from nor be inherited by
    // a mutable one.
    hazards.frozen |= fields.names.iter().any(|field| project.is_attribute_written(field))
        || class.bases().iter().any(|base| !is_generic(base))
        || project.is_base_class(name);
    // A class variable written through an instance becomes read-only once slots leave no `__dict__` to shadow it.
    if fields.class_variables.iter().any(|variable| project.is_attribute_written(variable)) {
        hazards.frozen = true;
        hazards.slots = true;
    }
    hazards.slots |= project.has_unslottable_subclass(qualified);
    hazards.slots |= decorated.wrapped;
    hazards.slots |= fields.defaults.iter().any(|field| project.is_class_attribute_accessed(qualified, field));
    // Only a frozen slotted dataclass gets a `__getstate__`: without one, pickle protocols 0 and 1 refuse a slotted
    // instance, and with one, a pickled subclass instance keeps only the fields. The fixes of both options apply
    // together, so the state is the one after freezing, whichever fix goes first.
    let frozen = decorated.option("frozen").is_some_and(is_true_literal) || !hazards.frozen;
    hazards.slots |= if frozen { project.has_stateful_subclass(qualified) } else { project.pickles_low_protocol() };
    let explicit =
        ["frozen", "slots"].iter().any(|option| decorated.option(option).is_some_and(|value| !is_true_literal(value)));
    hazards.suspected |= explicit || !class.bases().is_empty() || project.has_namesake_hazards(qualified);
}

// The names the module calls inside `pytest.raises(...)` or `self.assertRaises(...)`: a test creating an instance there
// expects it to fail somewhere, as an unhashable dict key, and either option can change that.
pub(crate) fn expected_to_raise_classes(body: &[Stmt]) -> FxHashSet<&str> {
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
        let raises = with
            .items
            .iter()
            .any(|item| item.context_expr.as_call_expr().is_some_and(|call| is_raises_or_assert_raises(&call.func)));
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
    annotated_as(annotation, &["ClassVar", "InitVar"])
}

// The value of `arg` in the `field(...)` call a field's default is, the call `dataclasses.field` makes for its options.
pub(super) fn field_keyword<'a>(value: Option<&'a Expr>, arg: &str) -> Option<&'a Expr> {
    let call = value?.as_call_expr()?;
    if last_segment(&call.func) != Some("field") {
        return None;
    }
    call.arguments.find_keyword(arg).map(|keyword| &keyword.value)
}

// Whether the annotation names one of `names`, subscripted or quoted, such as `ClassVar[int]`.
fn annotated_as(annotation: &Expr, names: &[&str]) -> bool {
    if let Some(text) = annotation.as_string_literal_expr() {
        let text = text.value.to_str();
        return names.iter().any(|name| text.contains(name));
    }
    let annotation = annotation.as_subscript_expr().map_or(annotation, |subscript| &subscript.value);
    last_segment(annotation).is_some_and(|name| names.contains(&name))
}

fn is_generic(base: &Expr) -> bool {
    let base = base.as_subscript_expr().map_or(base, |subscript| &subscript.value);
    last_segment(base) == Some("Generic")
}

// What breaks once the class is frozen or slotted. A hazard the class shows for certain withholds the option it
// breaks; one that only might break it, such as a class body writing to something other than `self`, makes the fix
// unsafe.
#[expect(clippy::struct_excessive_bools, reason = "each flag is one finding of the walk")]
struct Hazards<'f, 'a> {
    fields: &'f FxHashSet<&'f str>,
    // The fields whose class attribute is their default, which slots replace with a descriptor.
    defaults: &'f FxHashSet<&'f str>,
    frozen: bool,
    slots: bool,
    mutated: bool,
    suspected: bool,
    // How many functions deep the walk is, so a write to the receiver is known to come from a method.
    depth: usize,
    // The receiver of the method being walked, which a nested def keeps: a write through it is an instance write.
    receiver: Option<&'a str>,
    // Whether the method being walked is a classmethod, whose receiver is the class rather than an instance.
    classmethod: bool,
    // The depth at which the innermost class nested in the dataclass is walked.
    nested: Option<usize>,
    // The first parameter of a method of a nested class, the instance that class's methods write.
    own: Option<&'a str>,
}

impl Hazards<'_, '_> {
    const SUSPECTED: [&str; 5] = ["__dict__", "__weakref__", "__setattr__", "__delattr__", "super"];

    // A frozen instance rejects every write, `__dict__` included, and a slotted one has no room for an attribute that is
    // not a field.
    fn written(&mut self, attribute: &str) {
        self.mutated = true;
        self.frozen = true;
        self.slots |= !self.fields.contains(attribute);
    }

    // A write to `cls` in a classmethod replaces a class attribute rather than writing an instance, so only a field is a
    // hazard for it.
    fn receiver_written(&mut self, attribute: &str) {
        if self.classmethod {
            self.class_access(attribute);
            if !self.fields.contains(attribute) {
                return;
            }
        }
        self.written(attribute);
    }

    fn class_access(&mut self, attribute: &str) {
        self.slots |= self.defaults.contains(attribute);
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
                let outer = (self.receiver, self.classmethod, self.own);
                if self.nested == Some(self.depth) {
                    self.receiver = None;
                    self.own = receiver(function);
                } else if self.depth == 0 {
                    self.receiver = receiver(function);
                    self.classmethod = is_classmethod(function);
                }
                self.depth += 1;
                visitor::walk_stmt(self, stmt);
                self.depth -= 1;
                (self.receiver, self.classmethod, self.own) = outer;
                return;
            }
            // A class nested in the dataclass has instances of its own, so the writes its methods make through their
            // first parameter are not the dataclass's; any other write is still checked.
            Stmt::ClassDef(_) => {
                let outer = self.nested;
                self.nested = Some(self.depth);
                visitor::walk_stmt(self, stmt);
                self.nested = outer;
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
                    self.receiver_written(attr.as_str());
                } else if self.own.is_none_or(|own| !is_name(value, own)) {
                    self.suspected = true;
                }
            }
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Store, .. }) if id.as_str() == "__slots__" => {
                self.slots = true;
            }
            Expr::Name(ast::ExprName { id, .. }) => self.mention(id.as_str()),
            Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                if self.classmethod && self.receiver.is_some_and(|receiver| is_name(value, receiver)) {
                    self.class_access(attr.as_str());
                }
                self.mention(attr.as_str());
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// Whether a field's value leaves it a class attribute: `dataclasses` deletes the class attribute of a `field(...)` that
// has no default.
fn has_default(value: Option<&Expr>) -> bool {
    let Some(value) = value else {
        return false;
    };
    value.as_call_expr().is_none_or(|call| last_segment(&call.func) != Some("field"))
        || field_keyword(Some(value), "default").is_some()
}
