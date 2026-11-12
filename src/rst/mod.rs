// The reStructuredText scanning shared by the docstring and documentation rules, so the syntax is read in one place.
mod blocks;
mod directives;
mod fields;
mod lines;
mod literals;
mod roles;

pub(crate) use blocks::classify_docstring;
pub(crate) use directives::{Directive, directives};
pub(crate) use fields::{field_list, field_marker};
pub(crate) use lines::{docstring_lines, indent_width};
pub(crate) use literals::{Cause, literal_findings};
pub(crate) use roles::{bare_roles, references};
