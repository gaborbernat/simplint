use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Identifier, Stmt};
use ruff_python_parser::parse_expression;
use ruff_python_semantic::{Binding, BindingKind};
use ruff_python_stdlib::builtins::is_exception;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::{ExprCollector, for_each_body, is_name};

// The `str` methods that read the text of an exception to tell it apart from others.
const TEXT_METHODS: [&str; 5] = ["startswith", "endswith", "find", "index", "count"];

pub fn except_handler<'a>(walker: &mut Walker<'_, 'a>, handler: &'a ast::ExceptHandler) {
    let checker = walker.checker;
    let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: Some(name), body, .. }) = handler
    else {
        return;
    };
    // Tests may assert on the messages they expect.
    if checker.is_test_module() {
        return;
    }
    let mut asserts = Asserts::default();
    asserts.visit_body(body);
    // Ruff's PT017 reports each `assert` that names the exception, so a comparison inside one is left to it.
    let mut tests =
        ExprCollector::new(|expr: &Expr| by_text(checker, name, expr).filter(|range| !asserts.contains(*range)));
    tests.visit_body(body);
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Match(ast::StmtMatch { subject, .. }) = stmt
                && is_message(checker, subject, name)
            {
                tests.found.push(subject.range());
            }
        }
    });
    for range in tests.found {
        checker.report(
            Rule::ErrorMessageMatching,
            range,
            "Telling exceptions apart by their message breaks when the text changes; catch a more specific type",
        );
    }
}

// Walked at any depth, as Ruff's PT017 walks them, so a nested `assert` is left to Ruff too.
#[derive(Default)]
struct Asserts(Vec<TextRange>);

impl Asserts {
    fn contains(&self, range: TextRange) -> bool {
        self.0.iter().any(|assert| assert.contains_range(range))
    }
}

impl<'a> Visitor<'a> for Asserts {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if let Stmt::Assert(_) = stmt {
            self.0.push(stmt.range());
        }
        visitor::walk_stmt(self, stmt);
    }
}

fn text_name(expr: &Expr) -> Option<&ast::ExprName> {
    match expr {
        Expr::Call(_) => str_arg(expr),
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) if matches!(attr.as_str(), "args" | "message") => {
            value.as_name_expr()
        }
        Expr::Subscript(ast::ExprSubscript { value, .. }) => {
            value.as_attribute_expr().filter(|attribute| attribute.attr.as_str() == "args")?.value.as_name_expr()
        }
        _ => None,
    }
}

fn str_arg(expr: &Expr) -> Option<&ast::ExprName> {
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr else {
        return None;
    };
    match &*arguments.args {
        [Expr::Name(name)] if is_name(func, "str") => Some(name),
        _ => None,
    }
}

// A name that resolves to nothing is the handler's when it has the handler's text: a lambda in the handler reads the
// name after the handler has unbound it.
fn is_bound_name(checker: &Checker, name: &ast::ExprName, bound: &Identifier) -> bool {
    let bindings = checker.semantic().resolved_bindings(name);
    if bindings.is_empty() {
        return name.id.as_str() == bound.as_str();
    }
    bindings.iter().any(|binding| binding.range == bound.range())
}

fn is_message(checker: &Checker, expr: &Expr, bound: &Identifier) -> bool {
    text_name(expr).is_some_and(|name| is_bound_name(checker, name, bound))
}

fn is_exception_name(checker: &Checker, name: &ast::ExprName) -> bool {
    let bindings = checker.semantic().resolved_bindings(name);
    !bindings.is_empty() && bindings.into_iter().all(|binding| is_exception_binding(checker, binding))
}

fn is_exception_binding(checker: &Checker, binding: &Binding) -> bool {
    match &binding.kind {
        BindingKind::BoundException | BindingKind::UnboundException(_) => true,
        BindingKind::Argument => checker
            .semantic()
            .parameter_annotation(binding)
            .is_some_and(|annotation| is_exception_type(checker, annotation, true)),
        _ => false,
    }
}

// A string annotation's names are read from the string, not from the file, so they are not resolved.
fn is_exception_type(checker: &Checker, annotation: &Expr, resolved: bool) -> bool {
    match annotation {
        Expr::Name(_) => is_exception_class_name(checker, annotation, resolved, &mut Vec::new()),
        Expr::StringLiteral(literal) => parse_expression(literal.value.to_str())
            .is_ok_and(|parsed| is_exception_type(checker, parsed.expr(), false)),
        Expr::BinOp(ast::ExprBinOp { left, op: ast::Operator::BitOr, right, .. }) => {
            let exception = |expr: &Expr| is_exception_type(checker, expr, resolved);
            (exception(left) || exception(right))
                && [left, right].into_iter().all(|member| member.is_none_literal_expr() || exception(member))
        }
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            let optional = if resolved {
                checker.semantic().match_typing_expr(value, "Optional")
            } else {
                is_name(value, "Optional")
            };
            optional && is_exception_type(checker, slice, resolved)
        }
        _ => false,
    }
}

// The index only knows the classes at the top of a module, so the file's own classes are read here. A class's bases
// are real expressions, so they resolve, while a string annotation's names do not. `seen` stops a cycle of classes
// that derive from each other.
fn is_exception_class_name(checker: &Checker, expr: &Expr, resolved: bool, seen: &mut Vec<TextRange>) -> bool {
    expr.as_name_expr().is_some_and(|name| {
        let builtin = if resolved {
            checker.semantic().is_builtin(expr, name.id.as_str())
        } else {
            !checker.semantic().is_bound_in_file(name.id.as_str())
        };
        if (builtin && is_exception(name.id.as_str(), checker.target_version().minor))
            || checker.project().is_exception_type(name.id.as_str())
        {
            return true;
        }
        let classes: Vec<&ast::StmtClassDef> = if resolved {
            checker
                .semantic()
                .resolved_bindings(name)
                .into_iter()
                .filter_map(|binding| checker.semantic().class_of_binding(binding))
                .collect()
        } else {
            checker.class_defs().iter().copied().filter(|class| class.name.as_str() == name.id.as_str()).collect()
        };
        classes.into_iter().any(|class| {
            if seen.contains(&class.range) {
                return false;
            }
            seen.push(class.range);
            class.bases().iter().any(|base| is_exception_class_name(checker, base, true, seen))
        })
    })
}

// A comparison is left alone when each of its other operands is the text of an exception too, since two exceptions
// differ by wording only.
fn by_text(checker: &Checker, bound: &Identifier, expr: &Expr) -> Option<TextRange> {
    let found = match expr {
        Expr::Compare(ast::ExprCompare { operands, .. }) => {
            operands.iter().any(|operand| is_message(checker, operand, bound))
                && operands.iter().any(|operand| {
                    !is_message(checker, operand, bound)
                        && !text_name(operand).is_some_and(|name| is_exception_name(checker, name))
                })
        }
        Expr::Call(ast::ExprCall { func, arguments, .. }) => {
            let text_method = func.as_attribute_expr().is_some_and(|attribute| {
                TEXT_METHODS.contains(&attribute.attr.as_str())
                    && str_arg(&attribute.value).is_some_and(|name| is_bound_name(checker, name, bound))
            });
            let pattern_search = ["search", "match", "fullmatch"]
                .into_iter()
                .any(|function| checker.semantic().resolves_to(func, &["re", function]))
                && arguments
                    .args
                    .iter()
                    .any(|argument| str_arg(argument).is_some_and(|name| is_bound_name(checker, name, bound)));
            text_method || pattern_search
        }
        _ => false,
    };
    found.then(|| expr.range())
}
