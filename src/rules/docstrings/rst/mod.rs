// Shared parts of the reStructuredText docstring rules: field parsing, targets, attributes and fix edits.
mod attributes;
mod edits;
mod parse;
mod targets;

pub(crate) use attributes::has_attribute_bases;
pub(crate) use edits::field_deletion;
pub(crate) use parse::{Field, FieldKind, SIGNATURE_SECTIONS, field_marker};
pub(crate) use targets::{Owner, Target, collect_targets, is_internal};
