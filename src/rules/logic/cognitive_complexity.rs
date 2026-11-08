use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;

pub fn cognitive_complexity(checker: &Checker) {
    let limit = checker.settings.project.knobs.max_cognitive_complexity;
    for function in checker.function_defs() {
        let total = score(&function.body, 0);
        if total > limit {
            checker.report(
                Rule::CognitiveComplexity,
                function.name.range(),
                format!(
                    "`{}` has cognitive complexity {total}, over the limit of {limit}; split it into named helpers",
                    function.name
                ),
            );
        }
    }
}

// The score of `body` when it sits `nesting` bodies deep, where each nesting-sensitive increment grows by that depth.
// An inlined body adds the score it has at the depth of the call it replaces.
pub fn score(body: &[Stmt], nesting: usize) -> usize {
    Score::of(body, nesting).total
}

// The score of `body` together with the number of bodies the expression at `range` sits in, the nesting an inlined
// body would start at. The nesting is `None` when the walk does not reach `range`.
pub fn score_probed(body: &[Stmt], range: TextRange) -> (usize, Option<usize>) {
    let mut score = Score { probe: Some(range), ..Score::default() };
    score.visit_body(body);
    (score.total, score.probed)
}

// The SonarSource score of one function body. A branch, loop, handler or conditional expression adds one, plus one for
// each level of nesting it sits in; `elif` and `else` add one without the nesting; each run of one boolean operator
// adds one. As in SonarSource's Python analyzer, only the bodies nest, so a condition counts at the level of its
// statement, and comprehensions add nothing. A nested function is scored on its own, so its body is left out.
#[derive(Default)]
struct Score {
    total: usize,
    nesting: usize,
    // The boolean operator the expression being walked is a direct operand of, which a run of the same operator shares.
    boolean: Option<ast::BoolOp>,
    // The expression whose nesting the walk records, for a call site that an inlined helper would move to.
    probe: Option<TextRange>,
    probed: Option<usize>,
}

impl Score {
    fn of(body: &[Stmt], nesting: usize) -> Self {
        let mut score = Score { nesting, ..Score::default() };
        score.visit_body(body);
        score
    }

    fn nested(&mut self, visit: impl FnOnce(&mut Self)) {
        self.nesting += 1;
        visit(self);
        self.nesting -= 1;
    }

    fn branch(&mut self) {
        self.total += 1 + self.nesting;
    }

    fn nested_body(&mut self, body: &[Stmt]) {
        self.nested(|score| score.visit_body(body));
    }

    // The `else` of a loop or a `try` adds one, as the `else` of an `if` does.
    fn else_body(&mut self, body: &[Stmt]) {
        if !body.is_empty() {
            self.total += 1;
            self.nested_body(body);
        }
    }
}

impl<'a> Visitor<'a> for Score {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::If(ast::StmtIf { test, body, elif_else_clauses, .. }) => {
                self.branch();
                self.visit_expr(test);
                self.nested_body(body);
                for clause in elif_else_clauses {
                    self.total += 1;
                    if let Some(test) = &clause.test {
                        self.visit_expr(test);
                    }
                    self.nested_body(&clause.body);
                }
            }
            Stmt::For(ast::StmtFor { target, iter, body, orelse, .. }) => {
                self.branch();
                self.visit_expr(target);
                self.visit_expr(iter);
                self.nested_body(body);
                self.else_body(orelse);
            }
            Stmt::While(ast::StmtWhile { test, body, orelse, .. }) => {
                self.branch();
                self.visit_expr(test);
                self.nested_body(body);
                self.else_body(orelse);
            }
            Stmt::Match(ast::StmtMatch { subject, cases, .. }) => {
                self.branch();
                self.visit_expr(subject);
                self.nested(|score| cases.iter().for_each(|case| score.visit_match_case(case)));
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                self.visit_body(body);
                for handler in handlers {
                    self.branch();
                    self.nested(|score| score.visit_except_handler(handler));
                }
                self.else_body(orelse);
                self.visit_body(finalbody);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.probe == Some(expr.range()) {
            self.probed = Some(self.nesting);
        }
        let outer = match expr {
            Expr::BoolOp(ast::ExprBoolOp { op, .. }) => {
                if self.boolean != Some(*op) {
                    self.total += 1;
                }
                self.boolean.replace(*op)
            }
            _ => self.boolean.take(),
        };
        match expr {
            Expr::If(_) => {
                self.branch();
                self.nested(|score| visitor::walk_expr(score, expr));
            }
            Expr::Lambda(_) => self.nested(|score| visitor::walk_expr(score, expr)),
            _ => visitor::walk_expr(self, expr),
        }
        self.boolean = outer;
    }
}
