use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn expression_complexity(checker: &Checker) {
    let mut roots = Roots { checker, limit: checker.settings.project.knobs.max_expression_complexity };
    roots.visit_body(checker.body());
}

// Walks the statements and scores each expression a statement holds as a whole, so a nested part is not scored again.
struct Roots<'c, 'a> {
    checker: &'c Checker<'a>,
    limit: usize,
}

impl<'a> Visitor<'a> for Roots<'_, 'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            // A type alias is a type expression, which reads as a type rather than as a computation.
            Stmt::TypeAlias(_) => {}
            // Ruff's PLR0916 reports a test that joins more boolean operands than its limit.
            Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) => {
                self.visit_test(test);
                self.visit_body(body);
                for clause in elif_else_clauses {
                    if let Some(test) = &clause.test {
                        self.visit_test(test);
                    }
                    self.visit_body(&clause.body);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        let score = Depth::of(expr);
        if score > self.limit {
            self.checker.report(
                Rule::ExpressionComplexity,
                expr.range(),
                format!(
                    "Expression has complexity {score}, over the limit of {}; name its parts with variables or helpers",
                    self.limit
                ),
            );
        }
    }

    // Annotations are type expressions, scored by no one.
    fn visit_annotation(&mut self, _: &'a Expr) {}
}

impl<'a> Roots<'_, 'a> {
    fn visit_test(&mut self, test: &'a Expr) {
        if test
            .as_bool_op_expr()
            .is_none_or(|bool_op| boolean_operands(bool_op) <= self.checker.settings.from_ruff.max_bool_expr)
        {
            self.visit_expr(test);
        }
    }
}

// The operands Ruff's PLR0916 counts in a boolean test, nested `and` and `or` included.
fn boolean_operands(bool_op: &ast::ExprBoolOp) -> usize {
    bool_op.values.iter().map(|value| value.as_bool_op_expr().map_or(1, boolean_operands)).sum()
}

// The deepest nesting of weighted parts, a simplification of flake8-expression-complexity's ECE001: each part adds its
// weight to the operands below it. A name, a literal, an attribute and a collection display add nothing, and the left
// operand of the same binary operator continues its chain, as `a + b + c` reads as one sum.
#[derive(Default)]
struct Depth<'a> {
    current: usize,
    deepest: usize,
    chained: Option<&'a Expr>,
}

impl<'a> Depth<'a> {
    fn of(expr: &'a Expr) -> usize {
        let mut depth = Depth::default();
        depth.visit_expr(expr);
        depth.deepest
    }
}

impl<'a> Visitor<'a> for Depth<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let continued = self.chained.take().is_some_and(|chained| std::ptr::eq(chained, expr));
        let weight = if continued { 0 } else { weight(expr) };
        self.current += weight;
        self.deepest = self.deepest.max(self.current);
        if let Expr::BinOp(ast::ExprBinOp { left, op, .. }) = expr
            && left.as_bin_op_expr().is_some_and(|inner| inner.op == *op)
        {
            self.chained = Some(left);
        }
        visitor::walk_expr(self, expr);
        self.current -= weight;
    }
}

fn weight(expr: &Expr) -> usize {
    match expr {
        Expr::BoolOp(_)
        | Expr::BinOp(_)
        | Expr::UnaryOp(_)
        | Expr::Compare(_)
        | Expr::If(_)
        | Expr::Lambda(_)
        | Expr::Named(_)
        | Expr::Call(_)
        | Expr::Subscript(_)
        | Expr::ListComp(_)
        | Expr::SetComp(_)
        | Expr::DictComp(_)
        | Expr::Generator(_) => 1,
        _ => 0,
    }
}
