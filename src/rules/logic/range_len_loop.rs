use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::comparable::ComparableExpr;
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_stdlib::keyword::is_keyword;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::name_uses::{NameUses, ScopeNodes, scopes};

pub fn range_len_loop(checker: &Checker) {
    for (body, uses) in scopes(checker.body(), checker.function_defs()) {
        let nodes = ScopeNodes::of(body);
        for stmt in &nodes.statements {
            if let Stmt::For(loop_stmt) = stmt {
                check_loop(checker, loop_stmt, &uses, &nodes);
            }
        }
    }
}

fn check_loop(checker: &Checker, loop_stmt: &ast::StmtFor, uses: &NameUses, nodes: &ScopeNodes) {
    if loop_stmt.is_async {
        return;
    }
    let Expr::Name(index) = loop_stmt.target.as_ref() else {
        return;
    };
    let Some(sequence) = range_of_len(checker, &loop_stmt.iter) else {
        return;
    };
    let body = TextRange::new(loop_stmt.body[0].start(), loop_stmt.body.last().map_or(loop_stmt.end(), Ranged::end));
    let reads: Vec<&ast::ExprSubscript> = nodes
        .expressions
        .iter()
        .filter_map(|expr| expr.as_subscript_expr())
        .filter(|subscript| {
            body.contains_range(subscript.range)
                && subscript.ctx == ExprContext::Load
                && is_index(&subscript.slice, index)
                && ComparableExpr::from(subscript.value.as_ref()) == ComparableExpr::from(sequence)
        })
        .collect();
    if reads.is_empty() {
        return;
    }
    let sequence_text = checker.text(sequence);
    let fix = item_name(checker, sequence, uses).and_then(|item| {
        fixable(index, sequence, &reads, uses, nodes, body).then(|| {
            let mut edits = vec![
                Edit::range_replacement(item.clone(), index.range),
                Edit::range_replacement(sequence_text.to_owned(), loop_stmt.iter.range()),
            ];
            edits.extend(reads.iter().map(|read| Edit::range_replacement(item.clone(), read.range)));
            edits.sort_by_key(Edit::start);
            let first = edits.remove(0);
            Fix::unsafe_edits(first, edits)
        })
    });
    checker.report_fix(
        Rule::RangeLenLoop,
        TextRange::new(loop_stmt.start(), loop_stmt.iter.end()),
        format!("Loop over `{sequence_text}` itself instead of its indexes"),
        fix,
        format!("Loop over `{sequence_text}`"),
    );
}

// The `seq` of `range(len(seq))`, when it is a name or an attribute of one, which the loop can read again unchanged.
fn range_of_len<'a>(checker: &Checker, iter: &'a Expr) -> Option<&'a Expr> {
    let semantic = checker.semantic();
    let range_call = iter.as_call_expr()?;
    if !semantic.is_builtin(&range_call.func, "range") || !range_call.arguments.keywords.is_empty() {
        return None;
    }
    let [Expr::Call(len_call)] = range_call.arguments.args.as_ref() else {
        return None;
    };
    if !semantic.is_builtin(&len_call.func, "len") || !len_call.arguments.keywords.is_empty() {
        return None;
    }
    let [sequence] = len_call.arguments.args.as_ref() else {
        return None;
    };
    is_plain_reference(sequence).then_some(sequence)
}

fn is_plain_reference(expr: &Expr) -> bool {
    match expr {
        Expr::Name(_) => true,
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_plain_reference(value),
        _ => false,
    }
}

fn is_index(slice: &Expr, index: &ast::ExprName) -> bool {
    slice.as_name_expr().is_some_and(|name| name.id == index.id)
}

// The index is read only to subscript the sequence, the loop binds it alone, and nothing in the body rebinds or
// changes the sequence, so iterating the sequence gives the same items.
fn fixable(
    index: &ast::ExprName,
    sequence: &Expr,
    reads: &[&ast::ExprSubscript],
    uses: &NameUses,
    nodes: &ScopeNodes,
    body: TextRange,
) -> bool {
    let name = index.id.as_str();
    let index_loads = uses.loads(name);
    let root = root_name(sequence);
    uses.is_single_binding(name)
        && index_loads.len() == reads.len()
        && index_loads.iter().all(|load| reads.iter().any(|read| read.slice.range() == load.range))
        && !uses.is_opaque(root)
        && !uses.stores(root).iter().any(|store| body.contains_range(*store))
        && !nodes.expressions.iter().any(|expr| {
            body.contains_range(expr.range())
                && match expr {
                    Expr::Attribute(attribute) => {
                        ComparableExpr::from(attribute.value.as_ref()) == ComparableExpr::from(sequence)
                    }
                    Expr::Subscript(subscript) => {
                        ComparableExpr::from(subscript.value.as_ref()) == ComparableExpr::from(sequence)
                            && !reads.iter().any(|read| read.range == subscript.range)
                    }
                    _ => false,
                }
        })
}

fn root_name(expr: &Expr) -> &str {
    match expr {
        Expr::Attribute(ast::ExprAttribute { value, .. }) => root_name(value),
        Expr::Name(name) => name.id.as_str(),
        _ => "",
    }
}

// The singular of the sequence's name, `items` giving `item`, when the scope does not use it yet.
fn item_name(checker: &Checker, sequence: &Expr, uses: &NameUses) -> Option<String> {
    let last = match sequence {
        Expr::Attribute(attribute) => attribute.attr.as_str(),
        Expr::Name(name) => name.id.as_str(),
        _ => return None,
    };
    let singular = last.strip_suffix('s')?.trim_start_matches('_');
    let free = !singular.is_empty()
        && !singular.ends_with('s')
        && !is_keyword(singular)
        && uses.stores(singular).is_empty()
        && uses.loads(singular).is_empty()
        && !uses.is_opaque(singular)
        && !checker.semantic().is_bound_in_file(singular)
        && !ruff_python_stdlib::builtins::is_python_builtin(singular, checker.target_version().minor, false);
    free.then(|| singular.to_owned())
}
