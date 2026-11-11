use ruff_diagnostics::Fix;
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::{self as ast, BoolOp, CmpOp, Expr, Stmt, UnaryOp};
use ruff_text_size::Ranged;

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::modernize::{module_attribute_read, rename_attribute};

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    // `re.prefixmatch` is the same function as `re.match` from Python 3.15, so the rename keeps the behavior. A name
    // imported from `re` is not reported, since renaming it would also rename its uses. An assignment or `del` patches
    // the module, where a rename would patch the other name. A name bound to another module as well may be that one.
    let Some(attribute) = module_attribute_read(checker, expr, &["re", "match"]) else {
        return;
    };
    if left_to_ruff(walker, expr) {
        return;
    }
    let fix = Fix::safe_edit(rename_attribute(attribute, "prefixmatch"));
    checker.report_fix(
        Rule::RePrefixMatch,
        expr.range(),
        "`re.match` is soft deprecated from Python 3.15; `re.prefixmatch` names the same function",
        Some(fix),
        "Use `re.prefixmatch`",
    );
}

// Ruff's RUF055, in preview, rewrites a `re.match` call with a plain-text pattern whose result is only tested for truth
// into `str.startswith`, a fix that conflicts with this rule's, so such a call is left to it while it runs.
fn left_to_ruff(walker: &Walker, expr: &Expr) -> bool {
    let from_ruff = &walker.checker.settings.from_ruff;
    if !from_ruff.preview || !from_ruff.ruff_runs(RuffRule::UnnecessaryRegularExpression) {
        return false;
    }
    let expressions = walker.expressions();
    let [outer @ .., Expr::Call(call), _] = expressions else {
        return false;
    };
    std::ptr::eq(call.func.as_ref(), expr)
        && call.arguments.len() == 2
        && call.arguments.find_argument_value("string", 1).is_some()
        && call.arguments.find_argument_value("pattern", 0).is_some_and(|pattern| is_plain_text(walker, pattern))
        && (is_compared_to_none(outer.last().copied()) || is_boolean_test(walker, outer, expressions[outer.len()]))
}

// The pattern RUF055 reads: a string or bytes literal, or a name bound once to one, with no regular expression
// metacharacter in it.
fn is_plain_text(walker: &Walker, pattern: &Expr) -> bool {
    const METACHARACTERS: [char; 12] = ['.', '^', '$', '*', '+', '?', '{', '[', '\\', '|', '(', ')'];
    let literal = match pattern {
        Expr::Name(name) => walker.checker.semantic().only_binding_value(name),
        _ => Some(pattern),
    };
    match literal {
        Some(Expr::StringLiteral(string)) => !string.value.to_str().contains(METACHARACTERS),
        Some(Expr::BytesLiteral(bytes)) => {
            !bytes.value.iter().any(|part| part.iter().any(|&byte| METACHARACTERS.contains(&char::from(byte))))
        }
        _ => false,
    }
}

// `call is None` or `call is not None`, which RUF055 rewrites along with the call.
fn is_compared_to_none(parent: Option<&Expr>) -> bool {
    parent
        .and_then(Expr::as_compare_expr)
        .and_then(ast::ExprCompare::as_single)
        .is_some_and(|(_, op, right)| matches!(op, CmpOp::Is | CmpOp::IsNot) && right.is_none_literal_expr())
}

// Whether `expr` is a boolean test as Ruff's checker marks one: the test of an `if`, `elif`, `while`, `assert` or
// conditional expression, a comprehension's condition, a match guard, the operand of `not` or the argument of `bool()`,
// and the operands of `and` and `or` in such a test.
fn is_boolean_test<'a>(walker: &Walker<'_, 'a>, outer: &[&'a Expr], expr: &'a Expr) -> bool {
    let mut child = expr;
    for &parent in outer.iter().rev() {
        let is_child = |candidate: &Expr| std::ptr::eq(candidate, child);
        match parent {
            Expr::BoolOp(ast::ExprBoolOp { op: BoolOp::And | BoolOp::Or, .. }) => child = parent,
            Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, .. }) => return true,
            Expr::If(ast::ExprIf { test, .. }) => return is_child(test),
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                let semantic = walker.checker.semantic();
                return arguments.args.first().is_some_and(is_child)
                    && (semantic.is_builtin(func, "bool") || semantic.resolves_to(func, &["builtins", "bool"]));
            }
            Expr::ListComp(ast::ExprListComp { generators, .. })
            | Expr::SetComp(ast::ExprSetComp { generators, .. })
            | Expr::Generator(ast::ExprGenerator { generators, .. }) => return is_condition(generators, child),
            Expr::DictComp(ast::ExprDictComp { generators, .. }) => return is_condition(generators, child),
            _ => return false,
        }
    }
    let is_child = |candidate: &Expr| std::ptr::eq(candidate, child);
    match walker.statement() {
        Some(Stmt::If(ast::StmtIf { test, elif_else_clauses, .. })) => {
            is_child(test) || elif_else_clauses.iter().any(|clause| clause.test.as_ref().is_some_and(is_child))
        }
        Some(Stmt::While(ast::StmtWhile { test, .. }) | Stmt::Assert(ast::StmtAssert { test, .. })) => is_child(test),
        Some(Stmt::Match(ast::StmtMatch { cases, .. })) => {
            cases.iter().any(|case| case.guard.as_deref().is_some_and(is_child))
        }
        _ => false,
    }
}

fn is_condition(generators: &[ast::Comprehension], expr: &Expr) -> bool {
    generators.iter().any(|generator| generator.ifs.iter().any(|condition| std::ptr::eq(condition, expr)))
}
