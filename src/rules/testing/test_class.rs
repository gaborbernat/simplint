use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::any_over_body;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstring;
use crate::rules::testing::is_test_function;

pub fn test_class(checker: &Checker) {
    if !checker.is_test_module() {
        return;
    }
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
        let fix = (!referenced).then(|| flatten_class(checker, class)).flatten().map(|text| {
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

fn flatten_class(checker: &Checker, class: &ast::StmtClassDef) -> Option<String> {
    if class.arguments.as_ref().is_some_and(|arguments| !arguments.is_empty()) || !class.decorator_list.is_empty() {
        return None;
    }
    let source = checker.source;
    // Dedenting the methods would change the text of any string spanning several lines.
    if checker.parsed.tokens().in_range(class.range()).iter().any(|token| {
        matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle)
            && source.contains_line_break(token.range())
    }) {
        return None;
    }
    // Module-level functions have nowhere to keep the class docstring, and dropping it would lose its text.
    if docstring(&class.body).is_some() {
        return None;
    }
    let prefix = snake_case(class.name.as_str().strip_prefix("Test").unwrap_or_default());
    let mut functions = Vec::new();
    for stmt in &class.body {
        let Stmt::FunctionDef(function) = stmt else {
            return None;
        };
        let name = function.name.as_str();
        let rest = name.strip_prefix("test")?;
        let renamed = match (prefix.is_empty(), rest.is_empty() || rest.starts_with('_')) {
            (true, _) => name.to_owned(),
            (false, true) => format!("test_{prefix}{rest}"),
            (false, false) => format!("test_{prefix}_{rest}"),
        };
        if renamed != name && checker.name_token_count(&renamed) > 0 {
            return None;
        }
        let self_parameter =
            function.parameters.args.first().filter(|parameter| parameter.name().as_str() == "self")?;
        if any_over_body(&function.body, |expr| expr.as_name_expr().is_some_and(|name| name.id.as_str() == "self"))
            || !function.parameters.posonlyargs.is_empty()
        {
            return None;
        }
        let block = source.full_lines_range(TextRange::new(
            function.decorator_list.first().map_or(function.start(), Ranged::start),
            function.end(),
        ));
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
        text.push_str(&source[TextRange::new(function.name.end(), self_parameter.start())]);
        text.push_str(&source[TextRange::new(removal_end, block.end())]);
        functions.push(dedent_block(&text, checker.stylist.indentation().as_str()));
    }
    (!functions.is_empty()).then(|| functions.join("\n\n"))
}

fn dedent_block(text: &str, indent: &str) -> String {
    text.split_inclusive('\n').map(|line| line.strip_prefix(indent).unwrap_or(line)).collect()
}

fn snake_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut snake = String::new();
    for (index, char) in chars.iter().enumerate() {
        // A capital starts a word after a lowercase letter or digit, or ends an acronym when a lowercase letter follows:
        // `HTTPClient` becomes `http_client`.
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
