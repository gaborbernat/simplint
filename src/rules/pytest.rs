use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{
    appears_as_name, body_mentions, delete_statement, docstring, for_each_body, function_defs, is_alone_on_lines,
};

pub fn test_class(checker: &mut Checker) {
    if !checker.is_test_module() {
        return;
    }
    let mut found = Vec::new();
    for stmt in checker.body() {
        // pytest only collects classes named `Test*`; any other class holding a `test*` method is a helper.
        if let Stmt::ClassDef(class) = stmt
            && class.name.as_str().starts_with("Test")
            && class
                .body
                .iter()
                .filter_map(Stmt::as_function_def_stmt)
                .any(|function| function.name.as_str().starts_with("test"))
        {
            found.push(class);
        }
    }
    for class in found {
        let fix = flatten_class(checker, class).map(|text| {
            Fix::unsafe_edit(Edit::range_replacement(text, checker.source.full_lines_range(class.range())))
        });
        checker.report(
            Rule::TestClass,
            class.name.range(),
            format!("Test class `{}`; use module-level test functions with a name prefix", class.name.as_str()),
            fix,
        );
    }
}

fn flatten_class(checker: &Checker, class: &ast::StmtClassDef) -> Option<String> {
    if class.arguments.as_ref().is_some_and(|arguments| !arguments.is_empty()) || !class.decorator_list.is_empty() {
        return None;
    }
    let source = checker.source;
    // Dedenting the methods would change the text of any string spanning several lines.
    if checker.parsed.tokens().iter().any(|token| {
        class.range.contains_range(token.range())
            && matches!(token.kind(), TokenKind::String | TokenKind::FStringMiddle | TokenKind::TStringMiddle)
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
        if renamed != name && appears_as_name(checker, &renamed) {
            return None;
        }
        let self_parameter =
            function.parameters.args.first().filter(|parameter| parameter.name().as_str() == "self")?;
        if body_mentions(&function.body, "self", function.range) || !function.parameters.posonlyargs.is_empty() {
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

pub fn parametrize_without_id(checker: &mut Checker) {
    let mut found = Vec::new();
    for function in function_defs(checker.body()) {
        for decorator in &function.decorator_list {
            let Expr::Call(ast::ExprCall { func, arguments, .. }) = &decorator.expression else {
                continue;
            };
            if !UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().ends_with(&["mark", "parametrize"]))
            {
                continue;
            }
            let Some(values) = arguments.find_argument_value("argvalues", 1) else {
                continue;
            };
            if arguments.find_argument_value("ids", 3).is_some() {
                continue;
            }
            let (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })) = values else {
                continue;
            };
            found.extend(elts.iter().filter(|element| !is_param_with_id(element)).map(Ranged::range));
        }
    }
    for range in found {
        checker.report(
            Rule::ParametrizeWithoutId,
            range,
            "Parametrize row has no id; wrap it in `pytest.param(..., id=...)`",
            None,
        );
    }
}

fn is_param_with_id(element: &Expr) -> bool {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = element else {
        return false;
    };
    UnqualifiedName::from_expr(func).is_some_and(|name| name.segments().last() == Some(&"param"))
        && arguments.find_keyword("id").is_some()
}

pub fn conditional_in_test(checker: &mut Checker) {
    if !checker.is_test_module() {
        return;
    }
    let mut finder = Conditionals { found: Vec::new() };
    for function in function_defs(checker.body()) {
        if function.name.as_str().starts_with("test") {
            finder.visit_body(&function.body);
        }
    }
    let mut found = finder.found;
    found.sort_by_key(ruff_text_size::Ranged::start);
    found.dedup();
    for range in found {
        checker.report(Rule::ConditionalInTest, range, "Branching inside a test; parameterize instead", None);
    }
}

struct Conditionals {
    found: Vec<TextRange>,
}

impl<'a> Visitor<'a> for Conditionals {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::If(_) | Stmt::Match(_) => {
                self.found.push(stmt.range());
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => {}
            Expr::If(_) => {
                self.found.push(expr.range());
                visitor::walk_expr(self, expr);
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

pub fn constant_assert(checker: &mut Checker) {
    let mut found = Vec::new();
    for_each_body(checker.body(), &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Assert(assert) = stmt
                && is_truthy_constant(&assert.test)
            {
                found.push((stmt, assert, statements));
            }
        }
    });
    for (stmt, ast::StmtAssert { test, msg, .. }, statements) in found {
        let fix = match (test.as_ref(), msg) {
            // `assert (check, "message")` meant `assert check, "message"`; deleting it would drop the check.
            (Expr::Tuple(ast::ExprTuple { elts, .. }), None) if elts.len() == 2 => {
                let replacement =
                    format!("assert {}, {}", checker.text(elts[0].range()), checker.text(elts[1].range()));
                Some(Fix::unsafe_edit(Edit::range_replacement(replacement, stmt.range())))
            }
            (Expr::Tuple(_), _) => None,
            _ => is_alone_on_lines(checker.source, stmt.range())
                .then(|| Fix::safe_edit(delete_statement(checker.source, stmt, statements))),
        };
        checker.report(
            Rule::ConstantAssert,
            stmt.range(),
            "Assertion on a constant always passes; assert on the result under test",
            fix,
        );
    }
}

fn is_truthy_constant(expr: &Expr) -> bool {
    match expr {
        Expr::BooleanLiteral(literal) => literal.value,
        Expr::NumberLiteral(ast::ExprNumberLiteral { value, .. }) => match value {
            ast::Number::Int(int) => *int != ast::Int::ZERO,
            ast::Number::Float(float) => *float != 0.0,
            ast::Number::Complex { real, imag } => *real != 0.0 || *imag != 0.0,
        },
        Expr::StringLiteral(literal) => !literal.value.is_empty(),
        Expr::BytesLiteral(literal) => !literal.value.is_empty(),
        Expr::EllipsisLiteral(_) => true,
        Expr::Tuple(tuple) => !tuple.is_empty(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::TestClass, "tests/test_SMP200.py" ; "test_class")]
    #[test_case(Rule::TestClass, "SMP003_ok.py" ; "test_class_regular_module")]
    #[test_case(Rule::ParametrizeWithoutId, "tests/test_SMP201.py" ; "parametrize_without_id")]
    #[test_case(Rule::ConditionalInTest, "tests/test_SMP202.py" ; "conditional_in_test")]
    #[test_case(Rule::ConditionalInTest, "SMP003_ok.py" ; "conditional_in_test_regular_module")]
    #[test_case(Rule::ConstantAssert, "SMP205.py" ; "constant_assert")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }
}
