use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::{self as ast, Comprehension, Expr};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::registry::Rule;
use crate::lint::walker::Walker;
use crate::syntax::is_name;

// Python 3.15 unpacks inside a comprehension, so a nested `for` that only unpacks its iterable moves into the element.
pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let checker = walker.checker;
    let (generators, element) = match expr {
        Expr::ListComp(ast::ExprListComp { elt, generators, .. })
        | Expr::SetComp(ast::ExprSetComp { elt, generators, .. })
        | Expr::Generator(ast::ExprGenerator { elt, generators, .. }) => (generators.as_slice(), Element::Single(elt)),
        Expr::DictComp(ast::ExprDictComp { key: Some(key), value, generators, .. }) => {
            (generators.as_ref(), Element::Pair(key, value))
        }
        _ => return,
    };
    let [outer, inner] = generators else {
        return;
    };
    let Expr::Name(source) = &outer.target else {
        return;
    };
    let source = source.id.as_str();
    if outer.is_async
        || inner.is_async
        || !outer.ifs.is_empty()
        || !inner.ifs.is_empty()
        || is_name(&inner.target, source)
    {
        return;
    }
    let Some(unpacking) = unpacking(element, inner, source) else {
        return;
    };
    // The inner `for` clause runs from the outer iterable to the inner one, parentheses included.
    let clause = TextRange::new(
        checker.parenthesized_range(&outer.iter, outer.into()).end(),
        checker.parenthesized_range(&inner.iter, inner.into()).end(),
    );
    let element_range = match element {
        Element::Single(elt) => checker.parenthesized_range(elt, expr.into()),
        Element::Pair(key, value) => {
            checker.parenthesized_range(key, expr.into()).cover(checker.parenthesized_range(value, expr.into()))
        }
    };
    let fix = (!checker.has_comment_in(element_range) && !checker.has_comment_in(clause)).then(|| {
        let rewrite = Edit::range_replacement(unpacking.replacement(source), element_range);
        let remove = Edit::range_deletion(clause);
        if unpacking.safe { Fix::safe_edits(rewrite, [remove]) } else { Fix::unsafe_edits(rewrite, [remove]) }
    });
    let (message, title) = match unpacking.kind {
        Kind::Elements => (
            format!("`{source}` is unpacked by a nested `for`; unpack it in the element"),
            format!("Unpack `{source}` in the element"),
        ),
        Kind::Entries => (
            format!("`{source}.items()` is unpacked by a nested `for`; unpack `{source}` with `**` in the element"),
            format!("Unpack `{source}` with `**` in the element"),
        ),
    };
    checker.report_fix(Rule::ComprehensionUnpacking, expr.range(), message, fix, title);
}

#[derive(Clone, Copy)]
enum Element<'a> {
    Single(&'a Expr),
    Pair(&'a Expr, &'a Expr),
}

enum Kind {
    Elements,
    Entries,
}

struct Unpacking {
    kind: Kind,
    // A dict needs a real `dict`: `**` reads `keys()` and `__getitem__`, not `items()`.
    safe: bool,
}

impl Unpacking {
    fn replacement(&self, source: &str) -> String {
        match self.kind {
            Kind::Elements => format!("*{source}"),
            Kind::Entries => format!("**{source}"),
        }
    }
}

// The element must be the inner target, or, for a dict, the key and value unpacked from the pairs of `source.items()`.
fn unpacking(element: Element, inner: &Comprehension, source: &str) -> Option<Unpacking> {
    match element {
        Element::Single(elt) => {
            let (Expr::Name(elt), Expr::Name(target)) = (elt, &inner.target) else {
                return None;
            };
            (elt.id == target.id && is_name(&inner.iter, source))
                .then_some(Unpacking { kind: Kind::Elements, safe: true })
        }
        Element::Pair(key, value) => {
            let (Expr::Name(key), Expr::Name(value), Expr::Tuple(ast::ExprTuple { elts, .. })) =
                (key, value, &inner.target)
            else {
                return None;
            };
            let [Expr::Name(first), Expr::Name(second)] = elts.as_slice() else {
                return None;
            };
            let items = inner.iter.as_call_expr().is_some_and(|call| {
                call.arguments.is_empty()
                    && call.func.as_attribute_expr().is_some_and(|attribute| {
                        attribute.attr.as_str() == "items" && is_name(&attribute.value, source)
                    })
            });
            let paired = key.id == first.id && value.id == second.id && first.id != second.id;
            (items && paired).then_some(Unpacking { kind: Kind::Entries, safe: false })
        }
    }
}
