use ruff_python_ast::{self as ast, Comprehension, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let generators: &[Comprehension] = match expr {
        Expr::ListComp(ast::ExprListComp { generators, .. })
        | Expr::SetComp(ast::ExprSetComp { generators, .. })
        | Expr::Generator(ast::ExprGenerator { generators, .. }) => generators,
        Expr::DictComp(ast::ExprDictComp { generators, .. }) => generators,
        _ => return,
    };
    let Some((range, message)) = excess(generators) else {
        return;
    };
    walker.checker.report(Rule::ComprehensionComplexity, range, message);
}

fn excess(generators: &[Comprehension]) -> Option<(TextRange, String)> {
    // Two loops read as one pairing; a third is where the reader loses track of which loop each name belongs to.
    if let [_, _, third, ..] = generators {
        return Some((
            third.range(),
            format!(
                "Comprehension has {} `for` clauses; write the nested loop as `for` statements or a helper",
                generators.len()
            ),
        ));
    }
    let ifs = &generators[0].ifs;
    let [_, second, ..] = ifs.as_slice() else {
        return None;
    };
    Some((second.range(), format!("Comprehension has {} `if` clauses; name the condition in a helper", ifs.len())))
}
