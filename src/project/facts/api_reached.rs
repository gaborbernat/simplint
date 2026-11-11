use ruff_python_ast::{self as ast, Expr, Stmt};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::syntax::{Visibility, last_segment, module_statements, names_in};

// Only classes and aliases can be reached through an interface, so only they set `is_type`.
#[derive(Clone, rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub struct Definition {
    name: String,
    is_type: bool,
    refs: Vec<String>,
}

pub(super) fn interface(body: &[Stmt]) -> Vec<Definition> {
    module_statements(body).filter_map(definition).collect()
}

fn definition(stmt: &Stmt) -> Option<Definition> {
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
    Some(Definition { name: name.to_owned(), is_type, refs })
}

fn signature_names(function: &ast::StmtFunctionDef) -> Vec<String> {
    let mut refs = type_param_names(function.type_params.as_deref());
    let parameters = &function.parameters;
    refs.extend(
        parameters
            .posonlyargs
            .iter()
            .chain(&parameters.args)
            .chain(&parameters.kwonlyargs)
            .map(|parameter| &parameter.parameter)
            .chain(parameters.vararg.as_deref())
            .chain(parameters.kwarg.as_deref())
            .filter_map(|parameter| parameter.annotation())
            .flat_map(names_in),
    );
    refs.extend(function.returns.as_deref().into_iter().flat_map(names_in));
    refs
}

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

fn class_names(class: &ast::StmtClassDef) -> Vec<String> {
    let mut refs = type_param_names(class.type_params.as_deref());
    // Keywords such as `metaclass=` are left out.
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

// A root counts only when another root reaches it, so a type an exposed name uses is public though no file imports
// it. Built once per module, so the roots of several sets share the same references.
pub struct Closure<'a> {
    refs: FxHashMap<&'a str, Vec<&'a str>>,
    types: FxHashSet<&'a str>,
}

impl<'a> Closure<'a> {
    pub fn new(interface: &'a [Definition]) -> Self {
        let mut refs: FxHashMap<&str, Vec<&str>> = FxHashMap::default();
        let mut types: FxHashSet<&str> = FxHashSet::default();
        for definition in interface {
            refs.entry(definition.name.as_str()).or_default().extend(definition.refs.iter().map(String::as_str));
            if definition.is_type {
                types.insert(definition.name.as_str());
            }
        }
        Self { refs, types }
    }

    pub fn reached<'r>(&self, roots: impl IntoIterator<Item = &'r str>) -> Vec<String> {
        let mut reached: FxHashSet<&str> = FxHashSet::default();
        for root in roots {
            // The root itself is reached only through a cycle back to it, which no other root causes.
            let mut seen: FxHashSet<&str> = FxHashSet::default();
            let mut pending: Vec<&str> = self.refs.get(root).into_iter().flatten().copied().collect();
            while let Some(name) = pending.pop() {
                if self.types.contains(name) && seen.insert(name) {
                    pending.extend(self.refs.get(name).into_iter().flatten().copied());
                }
            }
            reached.extend(seen.into_iter().filter(|name| *name != root));
        }
        reached.into_iter().map(str::to_owned).collect()
    }
}
