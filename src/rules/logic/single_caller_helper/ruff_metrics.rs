use ruff_python_ast::helpers::ReturnStatementVisitor;
use ruff_python_ast::statement_visitor::StatementVisitor;
use ruff_python_ast::{self as ast, ExceptHandler, Stmt};

// Ruff's `get_complexity_number` for C901, without the one it adds for the function itself.
pub(super) fn mccabe_complexity(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                1 + mccabe_complexity(body)
                    + elif_else_clauses
                        .iter()
                        .map(|clause| usize::from(clause.test.is_some()) + mccabe_complexity(&clause.body))
                        .sum::<usize>()
            }
            Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
                1 + mccabe_complexity(body) + mccabe_complexity(orelse)
            }
            Stmt::With(ast::StmtWith { body, .. }) | Stmt::ClassDef(ast::StmtClassDef { body, .. }) => {
                mccabe_complexity(body)
            }
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                let irrefutable =
                    cases.last().is_some_and(|case| case.guard.is_none() && case.pattern.is_irrefutable());
                cases.iter().map(|case| 1 + mccabe_complexity(&case.body)).sum::<usize>() - usize::from(irrefutable)
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                mccabe_complexity(body)
                    + usize::from(!orelse.is_empty())
                    + mccabe_complexity(orelse)
                    + mccabe_complexity(finalbody)
                    + handlers.iter().map(|handler| 1 + mccabe_complexity(handler_body(handler))).sum::<usize>()
            }
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) => 1 + mccabe_complexity(body),
            _ => 0,
        })
        .sum()
}

// Ruff's `num_branches` for PLR0912.
pub(super) fn ruff_branches(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                1 + ruff_branches(body)
                    + elif_else_clauses.len()
                    + elif_else_clauses.iter().map(|clause| ruff_branches(&clause.body)).sum::<usize>()
            }
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                cases.len() + cases.iter().map(|case| ruff_branches(&case.body)).sum::<usize>()
            }
            Stmt::With(ast::StmtWith { body, .. }) => ruff_branches(body),
            Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
                1 + ruff_branches(body) + usize::from(!orelse.is_empty()) + ruff_branches(orelse)
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                ruff_branches(body)
                    + usize::from(!orelse.is_empty())
                    + ruff_branches(orelse)
                    + usize::from(!finalbody.is_empty())
                    + ruff_branches(finalbody)
                    + handlers.iter().map(|handler| 1 + ruff_branches(handler_body(handler))).sum::<usize>()
            }
            _ => 0,
        })
        .sum()
}

// Ruff's `num_statements` for PLR0915, which follows Pylint's counts.
pub(super) fn ruff_statements(stmts: &[Stmt]) -> usize {
    stmts
        .iter()
        .map(|stmt| match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
                1 + ruff_statements(body)
                    + elif_else_clauses.iter().map(|clause| 1 + ruff_statements(&clause.body)).sum::<usize>()
            }
            Stmt::For(ast::StmtFor { body, orelse, .. }) => ruff_statements(body) + ruff_statements(orelse),
            Stmt::While(ast::StmtWhile { body, orelse, .. }) => 1 + ruff_statements(body) + ruff_statements(orelse),
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                1 + cases.iter().map(|case| 1 + ruff_statements(&case.body)).sum::<usize>()
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
                1 + ruff_statements(body)
                    + usize::from(!orelse.is_empty())
                    + ruff_statements(orelse)
                    + 2 * usize::from(!finalbody.is_empty())
                    + ruff_statements(finalbody)
                    + usize::from(handlers.len() > 1)
                    + handlers.iter().map(|handler| 1 + ruff_statements(handler_body(handler))).sum::<usize>()
            }
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) | Stmt::With(ast::StmtWith { body, .. }) => {
                1 + ruff_statements(body)
            }
            Stmt::Return(_) => 0,
            _ => 1,
        })
        .sum()
}

// Ruff's `ReturnStatementVisitor` count for PLR0911.
pub(super) fn ruff_returns(body: &[Stmt]) -> usize {
    let mut visitor = ReturnStatementVisitor::default();
    visitor.visit_body(body);
    visitor.returns.len()
}

fn handler_body(handler: &ExceptHandler) -> &[Stmt] {
    let ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { body, .. }) = handler;
    body
}
