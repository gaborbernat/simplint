mod accumulator_loop;
pub(crate) mod anchored_regex_match;
mod cognitive_complexity;
mod collection_built_by_statements;
mod duplicate_code;
mod early_assignment;
pub(crate) mod empty_collection_comparison;
pub(crate) mod error_message_matching;
mod forwarding_function;
mod inline;
mod inline_regex;
mod iterator_reuse;
mod kinds;
pub(crate) mod long_tuple_return;
mod modernize;
mod name_uses;
mod range_len_loop;
mod regex_pattern;
mod scoped_block;
mod single_caller_helper;
mod single_use_variable;
pub(crate) mod slice_affix_comparison;
pub(crate) mod string_concatenation;
mod super_forwarding_method;
mod unexplained_regex;
mod unnamed_regex_group;
pub(crate) mod unused_default;
pub(crate) mod up_argparse_suggest_on_error;
pub(crate) mod up_asyncio_iscoroutinefunction;
mod up_chdir_restore;
pub(crate) mod up_codecs_open;
pub(crate) mod up_comprehension_unpacking;
pub(crate) mod up_compression_modules;
mod up_context_var_token_reset;
pub(crate) mod up_deprecated_stdlib_alias;
pub(crate) mod up_event_loop_in_coroutine;
pub(crate) mod up_exc_info_exception;
mod up_object_sentinel;
pub(crate) mod up_re_error_alias;
pub(crate) mod up_re_prefixmatch;
pub(crate) mod up_removed_ast_node;
pub(crate) mod up_strptime_date;
pub(crate) mod up_traceback_exception_arguments;
pub(crate) mod up_utf8_encoding_default;
pub(crate) mod up_warnings_deprecated;
mod walrus_candidate;
mod wall_clock_duration;

pub(crate) use accumulator_loop::accumulator_loop;
pub(crate) use cognitive_complexity::cognitive_complexity;
pub(crate) use collection_built_by_statements::collection_built_by_statements;
pub(crate) use duplicate_code::duplicate_code;
pub(crate) use early_assignment::early_assignment;
pub(crate) use forwarding_function::forwarding_function;
pub(crate) use inline_regex::inline_regex;
pub(crate) use iterator_reuse::iterator_reuse;
pub(crate) use kinds::KindsCache;
pub(crate) use range_len_loop::range_len_loop;
pub(crate) use single_caller_helper::single_caller_helper;
pub(crate) use single_use_variable::single_use_variable;
pub(crate) use super_forwarding_method::super_forwarding_method;
pub(crate) use unexplained_regex::unexplained_regex;
pub(crate) use unnamed_regex_group::unnamed_regex_group;
pub(crate) use unused_default::unused_default;
pub(crate) use up_chdir_restore::up_chdir_restore;
pub(crate) use up_compression_modules::up_compression_modules;
pub(crate) use up_context_var_token_reset::up_context_var_token_reset;
pub(crate) use up_object_sentinel::up_object_sentinel;
pub(crate) use walrus_candidate::walrus_candidate;
pub(crate) use wall_clock_duration::wall_clock_duration;

use ruff_python_ast::{self as ast, Expr};

fn sole_name_target(assign: &ast::StmtAssign) -> Option<&ast::ExprName> {
    let [Expr::Name(target)] = assign.targets.as_slice() else {
        return None;
    };
    Some(target)
}
