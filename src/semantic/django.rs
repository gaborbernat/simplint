// Ported from Ruff's DJ012 (crates/ruff_linter/src/rules/flake8_django/rules/unordered_body_content_in_model.rs, MIT),
// which reads the model state at the class, so the class is judged where the builder visits it.
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_python_semantic::analyze::class::any_qualified_base_class;
use ruff_python_semantic::{Modules, SemanticModel};
use ruff_text_size::{Ranged, TextRange};

// The body statements of a Django model that DJ012 reports: each is out of the Django Style Guide's order, below a
// statement of a later section. A run of one section is reported at its first statement only.
pub(super) fn unordered_model_statements(semantic: &SemanticModel, class: &ast::StmtClassDef) -> Vec<TextRange> {
    if !semantic.seen_module(Modules::DJANGO) || !is_model(class, semantic) {
        return Vec::new();
    }
    let mut reported = Vec::new();
    let mut seen: Vec<ContentType> = Vec::new();
    let mut previous = None;
    for stmt in &class.body {
        let Some(content) = content_type(stmt, semantic) else {
            continue;
        };
        if previous == Some(content) {
            continue;
        }
        previous = Some(content);
        // A statement out of order is not kept as a section that a later one may follow.
        if seen.iter().any(|&earlier| earlier > content) {
            reported.push(stmt.range());
        } else {
            seen.push(content);
        }
    }
    reported
}

fn is_model(class: &ast::StmtClassDef, semantic: &SemanticModel) -> bool {
    any_qualified_base_class(class, semantic, |name| matches!(name.segments(), ["django", "db", "models", "Model"]))
}

fn is_model_field(func: &Expr, semantic: &SemanticModel) -> bool {
    semantic.resolve_qualified_name(func).is_some_and(|name| name.segments().starts_with(&["django", "db", "models"]))
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ContentType {
    FieldDeclaration,
    ManagerDeclaration,
    MetaClass,
    MagicMethod,
    SaveMethod,
    GetAbsoluteUrlMethod,
    CustomMethod,
}

fn content_type(stmt: &Stmt, semantic: &SemanticModel) -> Option<ContentType> {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
            if let Expr::Call(ast::ExprCall { func, .. }) = value.as_ref()
                && is_model_field(func, semantic)
            {
                return Some(ContentType::FieldDeclaration);
            }
            let target = targets.first()?;
            (target.as_name_expr()?.id == "objects").then_some(ContentType::ManagerDeclaration)
        }
        Stmt::ClassDef(class) => (class.name.as_str() == "Meta").then_some(ContentType::MetaClass),
        Stmt::FunctionDef(function) => Some(match function.name.as_str() {
            name if is_dunder(name) => ContentType::MagicMethod,
            "save" => ContentType::SaveMethod,
            "get_absolute_url" => ContentType::GetAbsoluteUrlMethod,
            _ => ContentType::CustomMethod,
        }),
        _ => None,
    }
}
