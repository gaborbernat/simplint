// Ported from the preview version of Ruff's F401 (pyflakes/rules/unused_import.rs, MIT). Imports that bind the same
// name can have different uses, as in `import a` and `import a.b`. Match their qualified names against the reads in
// this module. Exports and string type expressions can also use a name.
use ruff_python_ast::name::{QualifiedName, QualifiedNameBuilder};
use ruff_python_ast::{Expr, Stmt};
use ruff_python_semantic::{
    Binding, BindingId, BindingKind, Imported, NodeId, ResolvedReferenceId, Scope, SemanticModel,
};
use ruff_text_size::TextRange;
use rustc_hash::FxHashSet;

use crate::syntax::{annotated_names, export_names, type_string_words};

pub(super) fn unused_imports(model: &SemanticModel, body: &[Stmt], extend_generics: &[String]) -> FxHashSet<TextRange> {
    let scope = model.global_scope();
    if !scope.bindings().any(|(_, live)| is_import(model.binding(live))) {
        return FxHashSet::default();
    }
    let exports = export_names(body);
    let annotated = annotated_names(body);
    // String type expressions such as `cast("X", value)` can use an import without leaving a resolved reference.
    let words = type_string_words(body, extend_generics);
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
        let found =
            if scope.shadowed_bindings(live).nth(1).is_some() && has_simple_shadowed_bindings(model, scope, live) {
                refined_unused(model, scope, live, name, listed, annotated.contains(name))
            } else if listed || annotated.contains(name) || copied_references(model, scope, live).next().is_some() {
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

fn has_simple_shadowed_bindings(model: &SemanticModel, scope: &Scope, live: BindingId) -> bool {
    scope.shadowed_bindings(live).enumerate().all(|(index, shadow)| {
        if index > 0 && copied_references(model, scope, shadow).next().is_some() {
            return false;
        }
        let shadowed = model.binding(shadow);
        if index > 0 && shadowed.source.is_none_or(|node| !is_top_level(model, node)) {
            return false;
        }
        matches!(shadowed.kind, BindingKind::Import(_) | BindingKind::SubmoduleImport(_)) && !shadowed.is_alias()
    })
}

// Ruff copies earlier references onto a rebinding. This model recovers them from the chain of shadowed bindings.
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

// A deletion ends the chain only at module level. A conditional deletion may leave earlier references in scope.
fn ends_copies(model: &SemanticModel, binding: &Binding) -> bool {
    match binding.kind {
        BindingKind::Builtin | BindingKind::UnboundException(_) => true,
        BindingKind::Deletion => binding.source.is_some_and(|node| is_top_level(model, node)),
        _ => false,
    }
}

// The model does not track branches, so use the module body to identify unconditional statements.
fn is_top_level(model: &SemanticModel, node: NodeId) -> bool {
    model.parent_statement(node).is_none()
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
    if listed
        && let Some(first) = model.binding(live).as_any_import().map(|import| import.qualified_name().segments()[0])
    {
        marked.mark(model, &QualifiedName::user_defined(first));
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
    let left =
        qualified.segments().iter().zip(prototype.segments()).take_while(|(ours, theirs)| ours == theirs).count();
    (left, std::cmp::Reverse(qualified.segments().len()))
}

// Use the full attribute chain to distinguish `import a` from `import a.b`.
fn expand_to_qualified_name_attribute<'b>(model: &SemanticModel<'b>, mut expr_id: NodeId) -> QualifiedName<'b> {
    let mut builder = QualifiedNameBuilder::with_capacity(16);
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
