use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::syntax::{Visibility, last_segment, module_statements, names_in};

// The module-level names a module's public interface reaches: the classes and aliases that the annotations, bases and
// bounds of an exported name refer to, transitively, through the same places of every class and alias they reach. A
// name counts only when another exported name reaches it, so a type an exported name uses is public though no file
// imports it.
pub(super) fn api_reached(body: &[Stmt], exported: &[&str]) -> Vec<String> {
    let mut refs: FxHashMap<&str, Vec<String>> = FxHashMap::default();
    let mut types: FxHashSet<&str> = FxHashSet::default();
    for stmt in module_statements(body) {
        if let Some(definition) = definition(stmt) {
            refs.entry(definition.name).or_default().extend(definition.refs);
            if definition.is_type {
                types.insert(definition.name);
            }
        }
    }
    let mut reached: FxHashSet<&str> = FxHashSet::default();
    for root in exported {
        // The root itself is reached only through a cycle back to it, which no other exported name causes.
        let mut seen: FxHashSet<&str> = FxHashSet::default();
        let mut pending: Vec<&str> = refs.get(root).into_iter().flatten().map(String::as_str).collect();
        while let Some(name) = pending.pop() {
            if types.contains(name) && seen.insert(name) {
                pending.extend(refs.get(name).into_iter().flatten().map(String::as_str));
            }
        }
        reached.extend(seen.into_iter().filter(|name| name != root));
    }
    reached.into_iter().map(str::to_owned).collect()
}

// A module-level definition, with the names its interface refers to. Only classes and aliases can be reached, so only
// they set `is_type`.
struct Definition<'a> {
    name: &'a str,
    is_type: bool,
    refs: Vec<String>,
}

fn definition(stmt: &Stmt) -> Option<Definition<'_>> {
    let (name, is_type, refs) = match stmt {
        Stmt::FunctionDef(function) => (function.name.as_str(), false, signature_names(function)),
        Stmt::ClassDef(class) => (class.name.as_str(), true, class_names(class)),
        Stmt::TypeAlias(ast::StmtTypeAlias { name, type_params, value, .. }) => {
            let mut refs = type_param_names(type_params.as_deref());
            refs.extend(names_in(value));
            (name.as_name_expr()?.id.as_str(), true, refs)
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value: Some(value), .. })
            if last_segment(annotation) == Some("TypeAlias") =>
        {
            (target.as_name_expr()?.id.as_str(), true, names_in(value))
        }
        // A plain assignment is an alias when its value has the shape of a type expression, so `Offset = int` counts
        // while `LIMIT = compute(3)` does not.
        Stmt::Assign(ast::StmtAssign { targets, value, .. })
            if matches!(
                &**value,
                Expr::Name(_)
                    | Expr::Attribute(_)
                    | Expr::Subscript(_)
                    | Expr::BinOp(ast::ExprBinOp { op: ast::Operator::BitOr, .. })
            ) =>
        {
            let [target] = targets.as_slice() else {
                return None;
            };
            (target.as_name_expr()?.id.as_str(), true, names_in(value))
        }
        _ => return None,
    };
    Some(Definition { name, is_type, refs })
}

// The names a class puts in its interface: its type parameters' bounds, its bases, the annotations of its fields, and
// the signatures of its public methods. Keywords such as `metaclass=` are left out.
fn class_names(class: &ast::StmtClassDef) -> Vec<String> {
    let mut refs = type_param_names(class.type_params.as_deref());
    refs.extend(class.bases().iter().flat_map(names_in));
    for stmt in &class.body {
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => refs.extend(names_in(annotation)),
            Stmt::FunctionDef(method) if Visibility::of(method.name.as_str()) == Visibility::Public => {
                refs.extend(signature_names(method));
            }
            _ => {}
        }
    }
    refs
}

// The names a function's signature puts in its interface: its type parameters' bounds and defaults, its parameter
// annotations and its return annotation.
fn signature_names(function: &ast::StmtFunctionDef) -> Vec<String> {
    let mut refs = type_param_names(function.type_params.as_deref());
    let parameters = &function.parameters;
    let annotated = parameters
        .posonlyargs
        .iter()
        .chain(&parameters.args)
        .chain(&parameters.kwonlyargs)
        .map(|parameter| &parameter.parameter)
        .chain(parameters.vararg.as_deref())
        .chain(parameters.kwarg.as_deref());
    refs.extend(annotated.filter_map(|parameter| parameter.annotation()).flat_map(names_in));
    refs.extend(function.returns.as_deref().into_iter().flat_map(names_in));
    refs
}

// The bounds and defaults of type parameters.
fn type_param_names(type_params: Option<&ast::TypeParams>) -> Vec<String> {
    let Some(type_params) = type_params else {
        return Vec::new();
    };
    type_params
        .type_params
        .iter()
        .flat_map(|param| {
            let bound = match param {
                ast::TypeParam::TypeVar(ast::TypeParamTypeVar { bound, .. }) => bound.as_deref(),
                ast::TypeParam::TypeVarTuple(_) | ast::TypeParam::ParamSpec(_) => None,
            };
            bound.into_iter().chain(param.default())
        })
        .flat_map(names_in)
        .collect()
}
