mod module_variable_case;
pub(crate) mod single_letter_name;
mod unexported_public_name;

pub(crate) use module_variable_case::module_variable_case;
pub(crate) use unexported_public_name::unexported_public_name;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_stdlib::builtins::is_python_builtin;
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::syntax::all_literal;

fn rename_fix(checker: &Checker, old: &str, new: &str) -> Option<Fix> {
    let ranges = checker.semantic().name_ranges(old);
    // A module name that shadows a builtin, as in `zip = zip`, shares its token with the builtin it reads.
    if checker.name_token_count(new) > 0
        || is_python_builtin(old, checker.settings.from_ruff.target_version.minor, false)
        || checker.project().is_referenced(old)
        || ranges.is_empty()
        || checker.name_token_count(old) != ranges.len()
    {
        return None;
    }
    let exports = all_elements(checker.body());
    if appears_in_strings(checker, old, &exports) {
        return None;
    }
    let renamed_exports = exports
        .into_iter()
        .filter(|element| element.value.to_str() == old)
        .filter_map(|element| Some(element.value.iter().next()?.content_range()));
    let mut edits =
        ranges.into_iter().chain(renamed_exports).map(|range| Edit::range_replacement(new.to_owned(), range));
    Some(Fix::unsafe_edits(edits.next()?, edits))
}

// PLY's `lex` and `yacc` build a lexer and a parser from the names the calling module defines, so a module that imports
// either reads its grammar by name.
fn uses_ply(checker: &Checker) -> bool {
    checker.body().iter().any(|stmt| match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) => names.iter().any(|alias| is_ply_module(alias.name.as_str())),
        Stmt::ImportFrom(ast::StmtImportFrom { module, names, .. }) => {
            let module = module.as_ref().map_or("", ast::Identifier::as_str);
            is_ply_module(module) || names.iter().any(|alias| is_ply_module(&format!("{module}.{}", alias.name)))
        }
        _ => false,
    })
}

// PLY is often vendored, as `jsonpath_ng._ply`, so any module under a package named after it counts.
fn is_ply_module(module: &str) -> bool {
    let mut parts = module.rsplit('.');
    parts.next().is_some_and(|last| matches!(last, "lex" | "yacc")) && parts.any(|part| part.contains("ply"))
}

// The token list, the `t_` lexer rules and `p_` grammar productions, and the settings PLY reads off a grammar module.
fn is_ply_name(name: &str) -> bool {
    name.starts_with("t_")
        || name.starts_with("p_")
        || matches!(name, "tokens" | "literals" | "precedence" | "states" | "start")
}

// The text a string holds, not its source: `ruff format` rewrites escapes, as `\N{latin small letter x}` into
// `\N{LATIN SMALL LETTER X}`, so a word in the source can come or go while the value stays.
fn appears_in_strings(checker: &Checker, name: &str, exports: &[&ast::ExprStringLiteral]) -> bool {
    // A word is a run of letters, digits and underscores, so a name with any other character is never one.
    name.chars().all(is_word_char)
        && checker.string_values().iter().any(|(range, text)| {
            !exports.iter().any(|export| export.range().contains_range(*range)) && contains_word(text, name)
        })
}

// Searching for the word and checking the characters around each match is far cheaper than splitting the text into
// words, and a match inside a longer word cannot hide a whole one, which starts after a separator.
fn contains_word(text: &str, word: &str) -> bool {
    text.match_indices(word).any(|(start, _)| {
        !text[..start].chars().next_back().is_some_and(is_word_char)
            && !text[start + word.len()..].chars().next().is_some_and(is_word_char)
    })
}

fn is_word_char(char: char) -> bool {
    char.is_alphanumeric() || char == '_'
}

fn all_elements(body: &[Stmt]) -> Vec<&ast::ExprStringLiteral> {
    body.iter()
        .filter_map(all_literal)
        .flat_map(|(_, elements)| elements)
        .filter_map(Expr::as_string_literal_expr)
        .collect()
}
