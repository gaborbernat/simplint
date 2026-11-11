use std::collections::HashSet;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::any_over_body;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt, TypeParam};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::noqa::{ruff_ignores, suppresses};
use crate::lint::registry::Rule;
use crate::rules::edits::statement_block;
use crate::rules::testing::is_test_function;
use crate::syntax::{docstring, expr_mentions, is_name, names_in};

pub fn test_class(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
    let mut new_names = HashSet::new();
    for stmt in checker.body() {
        // pytest only collects classes named `Test*`; any other class holding a `test*` method is a helper.
        let Stmt::ClassDef(class) = stmt else {
            continue;
        };
        if !class.name.as_str().starts_with("Test")
            || !class.body.iter().filter_map(Stmt::as_function_def_stmt).any(is_test_function)
        {
            continue;
        }
        // A class another class extends or code refers to must stay a class.
        let name = class.name.as_str();
        let referenced = !checker.semantic().references_to(name).is_empty() || checker.project().is_base_class(name);
        let fix = (!referenced).then(|| flatten_class(checker, class, &mut new_names)).flatten().map(|text| {
            Fix::unsafe_edit(Edit::range_replacement(text, checker.source.full_lines_range(class.range())))
        });
        checker.report_fix(
            Rule::TestClass,
            class.name.range(),
            format!("Test class `{}`; use module-level test functions with a name prefix", class.name.as_str()),
            fix,
            format!("Turn the methods of `{name}` into module-level functions"),
        );
    }
}

fn flatten_class(checker: &Checker, class: &ast::StmtClassDef, new_names: &mut HashSet<String>) -> Option<String> {
    if !can_flatten(checker, class) {
        return None;
    }
    let type_params = class.type_params.as_deref().map_or(&[][..], |params| params.type_params.as_slice());
    let prefix = snake_case(class.name.as_str().strip_prefix("Test").unwrap_or_default());
    let blocks: Vec<TextRange> = class.body.iter().map(|stmt| statement_block(checker, stmt)).collect();
    // A comment that is neither in a method nor on the lines directly above one, such as one on the class line or apart
    // from the method below it by a blank line, has no place among the functions.
    if checker
        .comment_ranges()
        .comments_in_range(class.range())
        .iter()
        .any(|comment| !blocks.iter().any(|block| block.contains(comment.start())))
    {
        return None;
    }
    let mut functions = Vec::new();
    let mut names = HashSet::new();
    for (stmt, block) in class.body.iter().zip(blocks) {
        let Stmt::FunctionDef(function) = stmt else {
            return None;
        };
        let (text, name) =
            flatten_method(checker, Method { function, type_params, prefix: &prefix }, block, new_names)?;
        // Methods of one class can share a name too: `test_a` and `testa` in `TestSame` both become `test_same_a`.
        if !names.insert(name) {
            return None;
        }
        functions.push(text);
    }
    new_names.extend(names);
    (!functions.is_empty()).then(|| functions.join(&checker.line_ending().repeat(2)))
}

// Whether the class can become module-level functions without its text changing: it has no bases, decorators or
// docstring, no line of it is silenced, and no string in it spans lines outside a method's docstring.
fn can_flatten(checker: &Checker, class: &ast::StmtClassDef) -> bool {
    if class.arguments.as_ref().is_some_and(|arguments| !arguments.is_empty()) || !class.decorator_list.is_empty() {
        return false;
    }
    // Dedenting shortens the lines, so a `noqa` or `ruff: ignore` that silences E501 on one of them would be left
    // unused, which Ruff reports as RUF100. The class stays as it is.
    let silenced = checker.comment_ranges().comments_in_range(class.range()).iter().any(|comment| {
        let text = checker.text(*comment);
        suppresses(text, "E501") || ruff_ignores(text, ["E501", "line-too-long"])
    });
    if silenced {
        return false;
    }
    let source = checker.source;
    // Dedenting the methods would change the text of any string spanning several lines, except a method's docstring,
    // whose indentation tools strip as `inspect.cleandoc` does and `ruff format` rewrites the same way.
    let docstrings: Vec<TextRange> = class
        .body
        .iter()
        .filter_map(Stmt::as_function_def_stmt)
        .filter_map(|function| docstring(&function.body).map(Ranged::range))
        .collect();
    if checker.parsed.tokens().in_range(class.range()).iter().any(|token| {
        matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle)
            && source.contains_line_break(token.range())
            && !docstrings.iter().any(|docstring| docstring.contains_range(token.range()))
    }) {
        return false;
    }
    // Module-level functions have nowhere to keep the class docstring, and dropping it would lose its text.
    if docstring(&class.body).is_some() {
        return false;
    }
    true
}

struct Method<'a> {
    function: &'a ast::StmtFunctionDef,
    type_params: &'a [TypeParam],
    prefix: &'a str,
}

// Turns a test method of a test class into a module-level function, named for the class, dropping `self`. Returns the
// function's text and its name.
fn flatten_method(
    checker: &Checker,
    Method { function, type_params, prefix }: Method<'_>,
    block: TextRange,
    new_names: &HashSet<String>,
) -> Option<(String, String)> {
    let source = checker.source;
    let name = function.name.as_str();
    let rest = name.strip_prefix("test")?;
    let renamed = match (prefix.is_empty(), rest.is_empty() || rest.starts_with('_')) {
        (true, _) => name.to_owned(),
        (false, true) => format!("test_{prefix}{rest}"),
        (false, false) => format!("test_{prefix}_{rest}"),
    };
    // An unchanged name is the method's own token too, so only a module-level binding of it counts as a collision. The
    // names of earlier classes are not in the file yet, so the token count misses them.
    let collides = new_names.contains(&renamed)
        || if renamed == name {
            checker.semantic().bindings(name).iter().any(|binding| binding.module_scope)
        } else {
            checker.name_token_count(&renamed) > 0
        };
    if collides {
        return None;
    }
    let self_parameter = function.parameters.args.first().filter(|parameter| parameter.name().as_str() == "self")?;
    if any_over_body(&function.body, |expr| is_name(expr, "self")) || !function.parameters.posonlyargs.is_empty() {
        return None;
    }
    // Decorators and defaults are evaluated outside the function, where its type parameters are not bound.
    let mut evaluated_outside = function
        .decorator_list
        .iter()
        .map(|decorator| &decorator.expression)
        .chain(function.parameters.iter().filter_map(ast::AnyParameterRef::default));
    if evaluated_outside.any(|expr| type_params.iter().any(|param| expr_mentions(expr, param.name().as_str()))) {
        return None;
    }
    let used = used_type_params(function, type_params);
    // A default on a type parameter is 3.13 syntax, which a fix for an older target would leave invalid.
    if checker.target_version() < PythonVersion::PY313 && used.iter().any(|param| param.default().is_some()) {
        return None;
    }
    let header: Vec<&str> = used.into_iter().map(|param| &source[param.range()]).collect();
    let following = checker.parsed.tokens().after(self_parameter.end());
    let removal_end = match following.first() {
        Some(comma) if comma.kind() == TokenKind::Comma => following
            .iter()
            .skip(1)
            .find(|token| !matches!(token.kind(), TokenKind::NonLogicalNewline))
            .map_or(comma.end(), Ranged::start),
        _ => self_parameter.end(),
    };
    let mut text = String::new();
    text.push_str(&source[TextRange::new(block.start(), function.name.start())]);
    text.push_str(&renamed);
    if !header.is_empty() {
        text.push('[');
        text.push_str(&header.join(", "));
        text.push(']');
    }
    text.push_str(&source[TextRange::new(function.name.end(), self_parameter.start())]);
    text.push_str(&source[TextRange::new(removal_end, block.end())]);
    Some((dedent_block(&text, checker.indentation(function)), renamed))
}

// The class's type parameters that a method's signature, body or nested type aliases name, plus those that their bounds
// and defaults name, in declaration order. The `self` parameter is dropped, so its annotation does not count.
fn used_type_params<'a>(function: &ast::StmtFunctionDef, type_params: &'a [TypeParam]) -> Vec<&'a TypeParam> {
    let mut uses = NameUses::default();
    for parameter in function.parameters.iter().skip(1) {
        if let Some(annotation) = parameter.annotation() {
            uses.visit_annotation(annotation);
        }
    }
    if let Some(returns) = &function.returns {
        uses.visit_annotation(returns);
    }
    uses.visit_body(&function.body);
    let mut pending = uses.names;
    let mut named = HashSet::new();
    while let Some(name) = pending.pop() {
        let Some(param) = type_params.iter().find(|param| param.name().as_str() == name) else {
            continue;
        };
        if named.insert(param.name().as_str()) {
            let bound = match param {
                TypeParam::TypeVar(ast::TypeParamTypeVar { bound, .. }) => bound.as_deref(),
                TypeParam::TypeVarTuple(_) | TypeParam::ParamSpec(_) => None,
            };
            pending.extend(bound.into_iter().chain(param.default()).flat_map(names_in));
        }
    }
    type_params.iter().filter(|param| named.contains(param.name().as_str())).collect()
}

// The names a function's annotations and body refer to. A quoted annotation counts as the names it spells.
#[derive(Default)]
struct NameUses {
    names: Vec<String>,
}

impl<'a> Visitor<'a> for NameUses {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::TypeAlias(alias) = stmt {
            self.names.extend(names_in(&alias.value));
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.names.push(name.id.to_string());
        }
        visitor::walk_expr(self, expr);
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        self.names.extend(names_in(expr));
    }
}

fn dedent_block(text: &str, indent: &str) -> String {
    text.split_inclusive('\n').map(|line| line.strip_prefix(indent).unwrap_or(line)).collect()
}

fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut snake = String::new();
    for (index, char) in chars.iter().enumerate() {
        // A capital starts a word after a lowercase letter or digit, or ends an acronym when a lowercase letter
        // follows: `HTTPClient` becomes `http_client`.
        let starts_word = index > 0
            && char.is_uppercase()
            && (!chars[index - 1].is_uppercase() || chars.get(index + 1).is_some_and(|next| next.is_lowercase()));
        if starts_word {
            snake.push('_');
        }
        snake.extend(char.to_lowercase());
    }
    snake
}
