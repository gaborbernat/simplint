// Ruff's F401 over the module scope, ported from `pyflakes/rules/unused_import.rs` (MIT) under its preview behavior. A
// name bound more than once is matched against its imports by the qualified names its loads read, so a shadowed import
// may be unused too. The model's references are the loads and `del`s, but it does not copy them to a rebinding as Ruff
// does, so a name's reads are the references of its chain of bindings up to the first `del`, builtin or unbound
// exception. `__all__` and string annotations read names the model does not resolve, so they stand in for loads.
use ruff_python_ast::name::{QualifiedName, QualifiedNameBuilder};
use ruff_python_ast::{Expr, Stmt};
use ruff_python_semantic::{
    Binding, BindingId, BindingKind, Imported, NodeId, ResolvedReferenceId, Scope, SemanticModel,
};
use ruff_text_size::TextRange;
use rustc_hash::FxHashSet;

use crate::syntax::{annotated_names, export_names, type_string_words};

pub fn unused_imports(model: &SemanticModel, body: &[Stmt]) -> FxHashSet<TextRange> {
    let scope = model.global_scope();
    if !scope.bindings().any(|(_, live)| is_import(model.binding(live))) {
        return FxHashSet::default();
    }
    let exports = export_names(body);
    let annotated = annotated_names(body);
    // Ruff reads the names a string type expression spells, as `cast("X", value)` or a `TypeVar` bound does, and the
    // model does not resolve them, so a word of such a string counts as a read.
    let words = type_string_words(body);
    let mut unused = FxHashSet::default();
    for (name, live) in scope.bindings() {
        let binding = model.binding(live);
        if !is_import(binding) || binding.is_global() || binding.is_nonlocal() || binding.is_explicit_export() {
            continue;
        }
        if words.contains(name) {
            continue;
        }
        let listed = exports.contains(name);
        let read = listed || annotated.contains(name);
        let found =
            if scope.shadowed_bindings(live).nth(1).is_some() && has_simple_shadowed_bindings(model, scope, live) {
                refined_unused(model, scope, live, name, listed, annotated.contains(name))
            } else if read || copied_references(model, scope, live).next().is_some() {
                Vec::new()
            } else {
                vec![live]
            };
        unused.extend(found.into_iter().map(|id| model.binding(id).range));
    }
    unused
}

fn is_import(binding: &Binding) -> bool {
    matches!(binding.kind, BindingKind::Import(_) | BindingKind::FromImport(_) | BindingKind::SubmoduleImport(_))
}

// The bindings a `del` or builtin ends the copying of references at, as a rebinding does not copy from them. Ruff
// records no `del` in a branch, so a `del` that is not at the top level does not end it.
fn ends_copies(model: &SemanticModel, binding: &Binding) -> bool {
    match binding.kind {
        BindingKind::Builtin | BindingKind::UnboundException(_) => true,
        BindingKind::Deletion => binding.source.is_some_and(|node| is_top_level(model, node)),
        _ => false,
    }
}

// The references a name's live binding has, which a rebinding copies from the bindings it shadows.
fn copied_references<'m>(
    model: &'m SemanticModel,
    scope: &'m Scope,
    live: BindingId,
) -> impl Iterator<Item = ResolvedReferenceId> + 'm {
    scope
        .shadowed_bindings(live)
        .take_while(move |prior| !ends_copies(model, model.binding(*prior)))
        .flat_map(move |prior| model.binding(prior).references())
}

// The model records no branches, so only a statement of the module body surely runs before the import that shadows it.
fn is_top_level(model: &SemanticModel, node: NodeId) -> bool {
    model.parent_statement(node).is_none()
}

fn has_simple_shadowed_bindings(model: &SemanticModel, scope: &Scope, live: BindingId) -> bool {
    scope.shadowed_bindings(live).enumerate().all(|(index, shadow)| {
        let shadowed = model.binding(shadow);
        if index > 0 && copied_references(model, scope, shadow).next().is_some() {
            return false;
        }
        if index > 0 && shadowed.source.is_none_or(|node| !is_top_level(model, node)) {
            return false;
        }
        matches!(shadowed.kind, BindingKind::Import(_) | BindingKind::SubmoduleImport(_)) && !shadowed.is_alias()
    })
}

fn refined_unused(
    model: &SemanticModel,
    scope: &Scope,
    live: BindingId,
    name: &str,
    listed: bool,
    annotated: bool,
) -> Vec<BindingId> {
    let bindings: Vec<BindingId> = scope.shadowed_bindings(live).collect();
    let mut marked = Marked { used: vec![false; bindings.len()], bindings };
    if listed {
        let first = model.binding(live).as_any_import().map(|import| import.qualified_name().segments()[0]);
        if let Some(first) = first {
            marked.mark(model, &QualifiedName::user_defined(first));
        }
    }
    if annotated {
        marked.mark(model, &QualifiedName::user_defined(name));
    }
    for reference in copied_references(model, scope, live) {
        let expr_id = model.reference(reference).expression_id().expect("a read is recorded under the name it reads");
        marked.mark(model, &expand_to_qualified_name_attribute(model, expr_id));
    }
    marked.into_unused()
}

struct Marked {
    bindings: Vec<BindingId>,
    used: Vec<bool>,
}

impl Marked {
    fn mark(&mut self, model: &SemanticModel, prototype: &QualifiedName) {
        let best = best_match(model, &self.bindings, prototype).expect("a refined chain holds the live import");
        let best_import = model.binding(best).as_any_import().expect("a refined chain holds only imports");
        let best_name = best_import.qualified_name();
        for (binding, used) in self.bindings.iter().zip(self.used.iter_mut()) {
            if !*used
                && model.binding(*binding).as_any_import().is_some_and(|import| import.qualified_name() == best_name)
            {
                *used = true;
            }
        }
    }

    fn into_unused(self) -> Vec<BindingId> {
        self.bindings.into_iter().zip(self.used).filter_map(|(binding, used)| (!used).then_some(binding)).collect()
    }
}

// The import that shares the longest prefix with the prototype, and of the fewest segments among those.
fn best_match(model: &SemanticModel, bindings: &[BindingId], prototype: &QualifiedName) -> Option<BindingId> {
    bindings.iter().copied().max_by_key(|binding| rank_matches(model, *binding, prototype))
}

fn rank_matches(
    model: &SemanticModel,
    binding: BindingId,
    prototype: &QualifiedName,
) -> (usize, std::cmp::Reverse<usize>) {
    let import = model.binding(binding).as_any_import().expect("only imports are matched against a prototype");
    let qualified = import.qualified_name();
    let left = qualified.segments().iter().zip(prototype.segments()).take_while(|(x, y)| x == y).count();
    (left, std::cmp::Reverse(qualified.segments().len()))
}

// The qualified name of the maximal attribute chain that an expression starts, as `a.b.foo` for `a` in `a.b.foo()`.
fn expand_to_qualified_name_attribute<'b>(model: &SemanticModel<'b>, expr_id: NodeId) -> QualifiedName<'b> {
    let mut builder = QualifiedNameBuilder::with_capacity(16);
    let mut expr_id = expr_id;
    let name = model.expression(expr_id).and_then(Expr::as_name_expr).expect("a read is a name");
    builder.push(&name.id);
    while let Some(node_id) = model.parent_expression_id(expr_id) {
        let Some(attribute) = model.expression(node_id).and_then(|expr| expr.as_attribute_expr()) else {
            break;
        };
        builder.push(attribute.attr.as_str());
        expr_id = node_id;
    }
    builder.build()
}
