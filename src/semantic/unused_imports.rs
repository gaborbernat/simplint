// Ruff's F401 over the module scope, ported from `pyflakes/rules/unused_import.rs` (MIT) under its preview behavior. A
// name bound more than once is matched against its imports by the qualified names its loads read, so a shadowed import
// may be unused too. The model's references are the loads and `del`s, but it does not copy them to a rebinding as Ruff
// does, so a name's reads are the references of its chain of bindings up to the first `del`, builtin or unbound
// exception. `__all__` and string annotations read names the model does not resolve, so they stand in for loads.
use ruff_python_ast::Stmt;
use ruff_python_ast::name::{QualifiedName, QualifiedNameBuilder};
use ruff_python_semantic::{
    Binding, BindingId, BindingKind, Imported, NodeId, ResolvedReferenceId, Scope, SemanticModel,
};
use ruff_text_size::TextRange;
use rustc_hash::FxHashSet;

use crate::syntax::{Exports, annotated_names, dunder_all, string_values, string_words};

pub fn unused_imports(model: &SemanticModel, body: &[Stmt]) -> FxHashSet<TextRange> {
    let exports = dunder_all(body);
    if matches!(exports, Exports::Dynamic) {
        return FxHashSet::default();
    }
    let scope = model.global_scope();
    if !scope.bindings().any(|(_, live)| is_import(model.binding(live))) {
        return FxHashSet::default();
    }
    let annotated = annotated_names(body);
    // Ruff reads the names that a string type expression spells, as `cast("X", value)` or a `TypeVar` bound does, and the
    // model does not resolve them, so every word of any string of the module counts as a read.
    let strings = string_values(body);
    let words = string_words(&strings);
    let mut unused = FxHashSet::default();
    for (name, live) in scope.bindings() {
        let binding = model.binding(live);
        if !is_import(binding) || binding.is_global() || binding.is_nonlocal() || binding.is_explicit_export() {
            continue;
        }
        if words.contains(name) {
            continue;
        }
        let read = exports.lists(name) || annotated.contains(name);
        let found =
            if scope.shadowed_bindings(live).nth(1).is_some() && has_simple_shadowed_bindings(model, scope, live) {
                refined_unused(model, scope, live, name, exports.lists(name), annotated.contains(name))
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

// The bindings a `del` or builtin ends the copying of references at, as a rebinding does not copy from them.
fn ends_copies(binding: &Binding) -> bool {
    matches!(binding.kind, BindingKind::Builtin | BindingKind::Deletion | BindingKind::UnboundException(_))
}

// The references a name's live binding has, which a rebinding copies from the bindings it shadows.
fn copied_references<'m>(
    model: &'m SemanticModel,
    scope: &'m Scope,
    live: BindingId,
) -> impl Iterator<Item = ResolvedReferenceId> + 'm {
    scope
        .shadowed_bindings(live)
        .take_while(move |prior| !ends_copies(model.binding(*prior)))
        .flat_map(move |prior| model.binding(prior).references())
}

fn has_simple_shadowed_bindings(model: &SemanticModel, scope: &Scope, live: BindingId) -> bool {
    let Some(live_node) = model.binding(live).source else {
        return false;
    };
    scope.shadowed_bindings(live).enumerate().all(|(index, shadow)| {
        let shadowed = model.binding(shadow);
        if index > 0 && copied_references(model, scope, shadow).next().is_some() {
            return false;
        }
        if shadowed.source.is_none_or(|node| !model.dominates(node, live_node)) {
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
        let Some(expr_id) = model.reference(reference).expression_id() else {
            return vec![live];
        };
        let Some(prototype) = expand_to_qualified_name_attribute(model, expr_id) else {
            return vec![live];
        };
        marked.mark(model, &prototype);
    }
    marked.into_unused()
}

struct Marked {
    bindings: Vec<BindingId>,
    used: Vec<bool>,
}

impl Marked {
    fn mark(&mut self, model: &SemanticModel, prototype: &QualifiedName) {
        let Some(best) = best_match(model, &self.bindings, prototype) else {
            return;
        };
        let Some(best_import) = model.binding(best).as_any_import() else {
            return;
        };
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
    let Some(import) = model.binding(binding).as_any_import() else {
        unreachable!("only imports are matched against a prototype")
    };
    let qualified = import.qualified_name();
    let left = qualified.segments().iter().zip(prototype.segments()).take_while(|(x, y)| x == y).count();
    (left, std::cmp::Reverse(qualified.segments().len()))
}

// The qualified name of the maximal attribute chain that an expression starts, as `a.b.foo` for `a` in `a.b.foo()`.
fn expand_to_qualified_name_attribute<'b>(model: &SemanticModel<'b>, expr_id: NodeId) -> Option<QualifiedName<'b>> {
    let mut builder = QualifiedNameBuilder::with_capacity(16);
    let mut expr_id = expr_id;
    let name = model.expression(expr_id)?.as_name_expr()?;
    builder.push(&name.id);
    while let Some(node_id) = model.parent_expression_id(expr_id) {
        let Some(attribute) = model.expression(node_id).and_then(|expr| expr.as_attribute_expr()) else {
            break;
        };
        builder.push(attribute.attr.as_str());
        expr_id = node_id;
    }
    Some(builder.build())
}
