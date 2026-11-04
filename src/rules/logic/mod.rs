pub(crate) mod error_message_matching;
mod forwarding_function;
mod inline;
pub(crate) mod kinds;
mod single_caller_helper;
mod single_use_variable;
pub(crate) mod string_concatenation;
mod unused_default;
mod walrus_candidate;

pub(crate) use forwarding_function::forwarding_function;
pub(crate) use kinds::KindsCache;
pub(crate) use single_caller_helper::single_caller_helper;
pub(crate) use single_use_variable::single_use_variable;
pub(crate) use unused_default::unused_default;
pub(crate) use walrus_candidate::walrus_candidate;
