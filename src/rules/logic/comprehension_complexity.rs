use ruff_python_ast::{self as ast, Comprehension, Expr, PythonVersion};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::rules::logic::up_comprehension_unpacking::unpacks_only;

// A comprehension reads as one loop with one filter; a second `for` or a second `if` is a nested loop or a compound
// filter that a `for` statement or a named predicate states better.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let generators: &[Comprehension] = match expr {
        Expr::ListComp(ast::ExprListComp { generators, .. })
        | Expr::SetComp(ast::ExprSetComp { generators, .. })
        | Expr::Generator(ast::ExprGenerator { generators, .. }) => generators,
        Expr::DictComp(ast::ExprDictComp { generators, .. }) => generators,
        _ => return,
    };
    let checker = walker.checker;
    // From Python 3.15 up-comprehension-unpacking (SMP508) folds a nested `for` that only unpacks into the element.
    if checker.target_version() >= PythonVersion::PY315 && unpacks_only(expr) {
        return;
    }
    let Some((range, message)) = excess(generators) else {
        return;
    };
    checker.report(Rule::ComprehensionComplexity, range, message);
}

fn excess(generators: &[Comprehension]) -> Option<(TextRange, String)> {
    if let [_, second, ..] = generators {
        return Some((
            second.range(),
            format!(
                "Comprehension has {} `for` clauses; write the nested loop as `for` statements or a helper",
                generators.len()
            ),
        ));
    }
    let ifs = &generators.first()?.ifs;
    let [_, second, ..] = ifs.as_slice() else {
        return None;
    };
    Some((second.range(), format!("Comprehension has {} `if` clauses; name the condition in a helper", ifs.len())))
}
