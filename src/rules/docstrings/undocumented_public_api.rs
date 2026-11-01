use ruff_python_ast::{self as ast, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::docstring;
use crate::rules::docstrings::{ApiKind, PublicApi};

pub fn undocumented_public_api(checker: &Checker) {
    if checker.is_test_module() {
        return;
    }
    let body = checker.body();
    let api = PublicApi::of(checker, body);
    for item in &api.items {
        let documented = match item.stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { body, .. }) | Stmt::ClassDef(ast::StmtClassDef { body, .. }) => {
                docstring(body).is_some()
            }
            _ => variable_documented(checker, body, item.stmt),
        };
        if documented {
            continue;
        }
        let name = item.name;
        let message = match item.kind {
            ApiKind::Variable => format!(
                "Exported variable `{name}` is undocumented; add a docstring below it or a `#:` comment above it"
            ),
            ApiKind::Function => format!("Public function `{name}` has no docstring; document it for its users"),
            ApiKind::Class => format!("Public class `{name}` has no docstring; document it for its users"),
            ApiKind::Method => format!("Public method `{name}` has no docstring; document it for its users"),
            ApiKind::Property => format!("Public property `{name}` has no docstring; document it for its users"),
        };
        checker.report(Rule::UndocumentedPublicApi, item.range, message);
    }
}

// A variable is documented the two ways Sphinx autodoc reads: a string right after it, or `#:` comments right above.
fn variable_documented(checker: &Checker, body: &[Stmt], stmt: &Stmt) -> bool {
    let next = body.iter().skip_while(|candidate| candidate.range() != stmt.range()).nth(1);
    let docstring_below =
        next.is_some_and(|next| next.as_expr_stmt().is_some_and(|expr| expr.value.is_string_literal_expr()));
    let is_doc_comment = |comment: TextRange| checker.text(comment).starts_with("#:");
    docstring_below
        || checker.comment_above(stmt).is_some_and(is_doc_comment)
        || checker.trailing_comment(stmt).is_some_and(is_doc_comment)
}
