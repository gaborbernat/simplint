use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Expr, Stmt, UnaryOp};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{body_mentions, for_each_body, function_defs, is_alone_on_lines, needs_parentheses};

pub fn walrus_candidate(checker: &mut Checker) {
    let mut found = Vec::new();
    for_each_body(checker.body(), &mut |statements, owner| {
        if matches!(owner, Some(Stmt::ClassDef(_))) {
            return;
        }
        for pair in statements.windows(2) {
            if let [Stmt::Assign(assign), Stmt::If(if_stmt)] = pair
                && let Some(candidate) = candidate(checker, assign, if_stmt)
            {
                found.push(candidate);
            }
        }
    });
    for Candidate { assign, name, target, bare, single_use } in found {
        let value = checker.parenthesized_text(&assign.value, assign.into());
        let replacement = match (single_use, bare) {
            (true, false) if needs_parentheses(&assign.value) && !value.starts_with('(') => format!("({value})"),
            (true, _) => value.to_owned(),
            (false, true) => format!("{name} := {value}"),
            (false, false) => format!("({name} := {value})"),
        };
        let message = if single_use {
            format!("Inline single-use `{name}` into the `if` test")
        } else {
            format!("Assign `{name}` with `:=` inside the `if` test that consumes it")
        };
        let deletion = Edit::range_deletion(checker.source.full_lines_range(assign.range));
        checker.report(
            Rule::WalrusCandidate,
            assign.range,
            message,
            Some(Fix::safe_edits(deletion, [Edit::range_replacement(replacement, target)])),
        );
    }
}

struct Candidate<'a> {
    assign: &'a ast::StmtAssign,
    name: &'a str,
    target: TextRange,
    bare: bool,
    single_use: bool,
}

fn candidate<'a>(
    checker: &Checker<'a>,
    assign: &'a ast::StmtAssign,
    if_stmt: &'a ast::StmtIf,
) -> Option<Candidate<'a>> {
    let [Expr::Name(target)] = assign.targets.as_slice() else {
        return None;
    };
    let name = target.id.as_str();
    let value = assign.value.as_ref();
    if matches!(value, Expr::Lambda(_) | Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Named(_) | Expr::Starred(_))
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
        || body_mentions(checker.body(), name, value.range())
        || !is_alone_on_lines(checker.source, assign.range)
        || checker.comments().any(|comment| comment.start() >= assign.start() && comment.start() < if_stmt.start())
        || checker.text(value.range()).contains("\\\n")
    {
        return None;
    }
    let (first, bare) = leading_name(&if_stmt.test, true)?;
    if first.id.as_str() != name {
        return None;
    }
    let function = function_defs(checker.body())
        .into_iter()
        .filter(|function| function.range.contains_range(if_stmt.range))
        .min_by_key(|function| function.range.len());
    let scope = function.map_or_else(|| TextRange::up_to(checker.source.text_len()), Ranged::range);
    // Inlining deletes the binding, so any other reader of it, earlier in the scope (a closure, the previous loop
    // iteration), in another module, or through `global`/`nonlocal`, would lose it.
    let shared = match function {
        Some(function) => declares_outer(&function.body, name),
        None => checker.is_imported_elsewhere(name),
    };
    if shared
        || body_mentions(checker.body(), name, TextRange::new(scope.start(), assign.start()))
        || body_mentions(checker.body(), name, TextRange::new(if_stmt.end(), scope.end()))
    {
        return None;
    }
    Some(Candidate {
        assign,
        name,
        target: first.range,
        bare,
        single_use: !body_mentions(checker.body(), name, TextRange::new(first.end(), if_stmt.end())),
    })
}

fn declares_outer(body: &[Stmt], name: &str) -> bool {
    let mut found = false;
    for_each_body(body, &mut |statements, _| {
        found |= statements.iter().any(|stmt| match stmt {
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                names.iter().any(|declared| declared.as_str() == name)
            }
            _ => false,
        });
    });
    found
}

fn leading_name(test: &Expr, bare: bool) -> Option<(&ast::ExprName, bool)> {
    match test {
        Expr::Name(name) => Some((name, bare)),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => leading_name(operand, false),
        Expr::Compare(ast::ExprCompare { operands, .. }) => leading_name(operands.first()?, false),
        Expr::BoolOp(ast::ExprBoolOp { values, .. }) => leading_name(values.first()?, false),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test]
    fn walrus_candidate() {
        check(Rule::WalrusCandidate, "SMP100.py", PythonVersion::PY314);
    }
}
