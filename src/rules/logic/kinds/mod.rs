mod bindings;
mod declared;
mod members;
mod query;
mod tables;

use std::cell::{OnceCell, RefCell};
use std::rc::Rc;

use ruff_text_size::TextRange;
use rustc_hash::FxHashMap;

use crate::lint::walker::{Scope, Walker};

use self::bindings::ScopeBindings;
pub(in crate::rules::logic) use self::declared::class_named;
use self::members::Members;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Kind {
    Str,
    Int,
    Bytes,
    Path,
    // A `str` or `None`, as `str | None` declares.
    OptionalStr,
    // Iteration yields strings. For a mapping, both keys and values must be strings.
    StrItems,
    // The same, with each item an `int`.
    IntItems,
    // An instance of the module's class defined at this range.
    Instance(TextRange),
    // An instance of a class the module imports, which the code does not show, so an operator may run its own method.
    ImportedInstance,
}

pub(super) struct Kinds<'w, 'a> {
    walker: &'w Walker<'w, 'a>,
    // Enclosing scopes, outermost first, excluding the module. While resolving an attribute, use the scopes
    // around the assignment that supplies its value.
    scopes: Vec<Scope<'a>>,
    // Whether `scopes` are those around the spot, where the walker knows the comprehensions too.
    at_spot: bool,
    // The names whose bindings are being checked, with the scope they resolve in and the kind assumed for a binding
    // that reads the name it binds, as `text = text + "!"` does.
    resolving: Vec<(&'a str, TextRange, Option<Kind>)>,
    // The attributes whose values are being checked, by class and name, with the length of `resolving` then: one that
    // reads itself stays unknown.
    reading: Vec<(TextRange, &'a str, usize)>,
    // Minimum stack depth this result depends on. A recursive name lookup depends on its assumed kind at that
    // depth. A recursive attribute lookup depends on the depth where checking that attribute began.
    relies_on: usize,
}

// Kept for the file, since each `+` chain builds a `Kinds` of its own.
#[derive(Default)]
pub(crate) struct KindsCache<'a> {
    // The bindings of the names in the def, class or lambda at a range, the module's at the default range.
    scopes: RefCell<FxHashMap<TextRange, Rc<ScopeBindings<'a>>>>,
    // How the class at a range gives an attribute to its instances.
    members: RefCell<FxHashMap<(TextRange, &'a str), Rc<Members<'a>>>>,
    // Where the module writes or deletes an attribute of each name.
    attribute_writes: OnceCell<FxHashMap<&'a str, Vec<TextRange>>>,
    // Cache only results independent of assumptions about enclosing lookups. They can be reused throughout the
    // scope without repeatedly tracing a chain of self-referential assignments.
    resolved: RefCell<FxHashMap<(&'a str, TextRange), Option<Kind>>>,
}
