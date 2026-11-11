// Ported from Ruff's FURB113 (crates/ruff_linter/src/rules/refurb/rules/repeated_append.rs, MIT), whose `match_append`
// reads the scope as it stands at the statement, so the builder asks it there.
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_semantic::SemanticModel;
use ruff_python_semantic::analyze::typing::is_list;

pub(super) fn is_list_append(semantic: &SemanticModel, stmt: &Stmt) -> bool {
    let Stmt::Expr(ast::StmtExpr { value, .. }) = stmt else {
        return false;
    };
    let Expr::Call(ast::ExprCall { func, arguments, .. }) = value.as_ref() else {
        return false;
    };
    let Expr::Attribute(ast::ExprAttribute { value: receiver, attr, .. }) = func.as_ref() else {
        return false;
    };
    let Expr::Name(receiver) = receiver.as_ref() else {
        return false;
    };
    if arguments.args.len() != 1 || !arguments.keywords.is_empty() || attr.as_str() != "append" {
        return false;
    }
    let bindings: Vec<_> = semantic.current_scope().get_all(receiver.id.as_str()).collect();
    let [binding] = bindings.as_slice() else {
        return false;
    };
    is_list(semantic.binding(*binding), semantic)
}
