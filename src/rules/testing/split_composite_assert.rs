use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, CmpOp, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::layout::joined_width;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::testing::is_test_module_function;
use crate::syntax::for_each_body;

pub fn split_composite_assert(checker: &Checker) {
    let mut runs: Vec<&[Stmt]> = Vec::new();
    for function in checker.function_defs() {
        if is_test_module_function(checker, function) {
            for_each_body(&function.body, &mut |statements, _| collect_runs(checker, statements, &mut runs));
        }
    }
    // A test nested in another test is reached from both, so each run is kept once.
    runs.sort_by_key(|run| run[0].start());
    runs.dedup_by_key(|run| run[0].start());
    for run in runs {
        report(checker, run);
    }
}

struct Field<'a> {
    compare: &'a ast::ExprCompare,
    access: &'a Expr,
    owner: &'a Expr,
    value: &'a Expr,
}

// An assert with no message that compares one attribute or key of an object with `==`.
fn field_of(stmt: &Stmt) -> Option<Field<'_>> {
    let Stmt::Assert(ast::StmtAssert { test, msg: None, .. }) = stmt else {
        return None;
    };
    let Expr::Compare(compare) = test.as_ref() else {
        return None;
    };
    let ([CmpOp::Eq], [field, value]) = (compare.ops.as_ref(), compare.operands.as_ref()) else {
        return None;
    };
    let (Expr::Attribute(ast::ExprAttribute { value: owner, .. })
    | Expr::Subscript(ast::ExprSubscript { value: owner, .. })) = field
    else {
        return None;
    };
    Some(Field { compare, access: field, owner, value })
}

// A run is two or more consecutive statements whose fields belong to the same object.
fn collect_runs<'a>(checker: &Checker, statements: &'a [Stmt], runs: &mut Vec<&'a [Stmt]>) {
    let mut start = 0;
    while start < statements.len() {
        let Some(first) = field_of(&statements[start]) else {
            start += 1;
            continue;
        };
        let owner = checker.text(first.owner.range());
        let mut end = start + 1;
        while statements.get(end).and_then(field_of).is_some_and(|field| checker.text(field.owner.range()) == owner) {
            end += 1;
        }
        if end - start >= 2 {
            runs.push(&statements[start..end]);
        }
        start = end;
    }
}

fn report(checker: &Checker, run: &[Stmt]) {
    let fields: Vec<Field> = run.iter().filter_map(field_of).collect();
    let owner = checker.text(fields[0].owner.range());
    let names: Vec<&str> = fields.iter().map(|field| checker.text(field.access.range())).collect();
    let values: Vec<&str> =
        fields.iter().map(|field| checker.parenthesized_text(field.value, field.compare.into())).collect();
    let replacement = format!("assert ({}) == ({})", names.join(", "), values.join(", "));
    // A diagnostic is one line, so a value written over several lines is shown joined.
    let example = replacement.split_whitespace().collect::<Vec<_>>().join(" ");
    let message =
        format!("These asserts check fields of `{owner}` one by one; compare them in one assert, as in `{example}`");
    let range = TextRange::new(run[0].start(), run[run.len() - 1].end());
    let fix = fits(checker, run, range, &fields).then(|| Fix::unsafe_edit(Edit::range_replacement(replacement, range)));
    checker.report_fix(Rule::SplitCompositeAssert, run[0].range(), message, fix, "Compare the fields in one assert");
}

// The joined assert must fit the line it replaces: the line's indentation and the assert, joined on one line. Comments
// in the run would be lost, and the run must own its lines for the replacement to keep the statements' layout.
fn fits(checker: &Checker, run: &[Stmt], range: TextRange, fields: &[Field]) -> bool {
    if !checker.is_alone_on_lines(range) || checker.has_comment_in(range) {
        return false;
    }
    let (source, tokens) = (checker.source, checker.parsed.tokens());
    let names: Option<Vec<usize>> =
        fields.iter().map(|field| joined_width(source, tokens, field.access.range())).collect();
    let values: Option<Vec<usize>> = fields
        .iter()
        .map(|field| joined_width(source, tokens, checker.parenthesized_range(field.value, field.compare.into())))
        .collect();
    let (Some(names), Some(values)) = (names, values) else {
        return false;
    };
    // Each tuple's elements are separated by `, `.
    let joined = |widths: &[usize]| widths.iter().sum::<usize>() + ", ".len() * (widths.len() - 1);
    let width = "assert (".len() + joined(&names) + ") == (".len() + joined(&values) + ")".len();
    checker.indentation(&run[0]).len() + width <= checker.settings.from_ruff.line_width
}
