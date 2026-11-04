// The reStructuredText docstring rules share these parts: the parser of a docstring's field list, the docstrings to check
// with what each documents, the class model autodoc reads attributes from, and the edits their fixes make.
mod attributes;
mod edits;
mod parse;
mod targets;

pub(crate) use attributes::has_attribute_bases;
pub(crate) use edits::{field_deletion, field_insertion, rank};
pub(crate) use parse::{Docstring, Field, FieldKind};
pub(crate) use targets::{Owner, Parameter, Target, collect_targets, is_internal};
