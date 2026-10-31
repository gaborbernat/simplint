mod builder;

use std::path::Path;

use ruff_python_ast::identifier::Identifier;

use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_python_semantic::{BindingId, BindingKind as RuffBindingKind, ScopeKind, SemanticModel};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

pub struct Semantic<'a> {
    model: SemanticModel<'a>,
    annotations_are_lazy: bool,
    index: Index<'a>,
}

impl<'a> Semantic<'a> {
    pub fn new(path: &'a Path, body: &'a [Stmt], target_version: PythonVersion) -> Self {
        let builder::Model { semantic: mut model, loads, module, annotations_are_lazy } =
            builder::build(path, body, target_version);
        let mut index = Index::default();
        let module_bindings = index.collect_bindings(&model);
        for (name, snapshot) in loads {
            model.restore(snapshot);
            index.collect_load(&model, &module_bindings, body, name);
        }
        // Function bodies are visited last, so loads arrive out of source order.
        for references in index.references.values_mut() {
            references.sort_by_key(|reference| reference.range.start());
        }
        model.restore(module);
        Self { model, annotations_are_lazy, index }
    }

    // Whether `expr` names `segments`, resolved as Ruff does through the imports that bind its head, so `mock.patch`
    // after `from unittest import mock` names `unittest.mock.patch`.
    pub fn resolves_to(&self, expr: &Expr, segments: &[&str]) -> bool {
        self.model.resolve_qualified_name(expr).is_some_and(|name| name.segments() == segments)
    }

    // Whether `expr` names `typing.{member}`, or its `typing_extensions` backport.
    pub fn match_typing_expr(&self, expr: &Expr, member: &str) -> bool {
        self.model.match_typing_expr(expr, member)
    }

    // Whether annotations wait until something reads them: from `from __future__ import annotations` or a target
    // version that defers them.
    pub fn annotations_are_lazy(&self) -> bool {
        self.annotations_are_lazy
    }

    pub fn bindings(&self, name: &str) -> &[Binding] {
        self.index.bindings.get(name).map_or(&[], Vec::as_slice)
    }

    pub fn all_bindings(&self) -> impl Iterator<Item = (&'a str, &[Binding])> {
        self.index.bindings.iter().map(|(name, bindings)| (*name, bindings.as_slice()))
    }

    // In source order.
    pub fn references_to(&self, name: &str) -> &[Reference<'a>] {
        self.index.references.get(name).map_or(&[], Vec::as_slice)
    }

    pub fn references(&self) -> impl Iterator<Item = &Reference<'a>> {
        self.index.references.values().flatten()
    }

    // Where the methods of `class` read `name` off `self`, `cls` or the class.
    pub fn member_references(&self, class: TextRange, name: &str) -> &[TextRange] {
        self.index.member_references.get(&class).and_then(|members| members.get(name)).map_or(&[], Vec::as_slice)
    }

    pub fn is_rebound(&self, name: &str) -> bool {
        self.bindings(name).len() > 1
    }

    pub fn is_bound(&self, name: &str) -> bool {
        self.index.bindings.contains_key(name)
    }

    pub fn name_ranges(&self, name: &str) -> Vec<TextRange> {
        let mut ranges: Vec<TextRange> = self
            .bindings(name)
            .iter()
            .map(|binding| binding.range)
            .chain(self.references_to(name).iter().map(|reference| reference.range))
            .chain(self.index.global_declarations.get(name).into_iter().flatten().copied())
            .collect();
        ranges.sort_by_key(Ranged::start);
        ranges.dedup();
        ranges
    }
}

pub fn future_import(stmt: &Stmt) -> Option<&ast::StmtImportFrom> {
    stmt.as_import_from_stmt().filter(|import| import.module.as_ref().is_some_and(|module| module == "__future__"))
}

// The module-level names of `Semantic`, by name, so rules look them up instead of scanning.
#[derive(Default)]
struct Index<'a> {
    bindings: FxHashMap<&'a str, Vec<Binding>>,
    references: FxHashMap<&'a str, Vec<Reference<'a>>>,
    member_references: FxHashMap<TextRange, FxHashMap<&'a str, Vec<TextRange>>>,
    global_declarations: FxHashMap<&'a str, Vec<TextRange>>,
}

impl<'a> Index<'a> {
    // Returns every binding a module-level name resolves to, builtins and `global` declarations included.
    fn collect_bindings(&mut self, model: &SemanticModel<'a>) -> FxHashSet<BindingId> {
        let mut module_bindings = FxHashSet::default();
        for scope in model.scopes.iter() {
            for (name, latest) in scope.bindings() {
                let chain: Vec<BindingId> = scope.shadowed_bindings(latest).collect();
                let declarations: Vec<TextRange> = chain
                    .iter()
                    .map(|binding_id| model.binding(*binding_id))
                    .filter(|binding| binding.kind.is_global())
                    .map(|binding| binding.range)
                    .collect();
                if !scope.kind.is_module() && declarations.is_empty() {
                    continue;
                }
                if !declarations.is_empty() {
                    self.global_declarations.entry(name).or_default().extend(declarations);
                }
                let bound: Vec<Binding> = chain
                    .iter()
                    .filter(|binding_id| {
                        !matches!(
                            model.binding(**binding_id).kind,
                            RuffBindingKind::Global(_)
                                | RuffBindingKind::Builtin
                                | RuffBindingKind::UnboundException(_)
                        )
                    })
                    .flat_map(|binding_id| {
                        std::iter::once(binding_id).chain(model.delayed_annotations(*binding_id).into_iter().flatten())
                    })
                    .map(|binding_id| Binding {
                        range: binding_range(model, *binding_id),
                        kind: binding_kind(model, *binding_id),
                    })
                    .collect();
                module_bindings.extend(chain);
                if !bound.is_empty() {
                    self.bindings.entry(name).or_default().extend(bound);
                }
            }
        }
        for bindings in self.bindings.values_mut() {
            bindings.sort_by_key(|binding| binding.range.start());
        }
        module_bindings
    }

    fn collect_load(
        &mut self,
        model: &SemanticModel<'a>,
        module_bindings: &FxHashSet<BindingId>,
        body: &[Stmt],
        name: &'a ast::ExprName,
    ) {
        let function = model.current_scopes().filter_map(|scope| function_range(scope.kind)).last();
        if function.is_some()
            && let Some(class) = model.current_scopes().find_map(|scope| match scope.kind {
                ScopeKind::Class(class) => Some(class),
                _ => None,
            })
            && (matches!(name.id.as_str(), "self" | "cls") || name.id == class.name.as_str())
            && let Some(Expr::Attribute(attribute)) = model.current_expression_parent()
        {
            let members = self.member_references.entry(class.range).or_default();
            members.entry(attribute.attr.as_str()).or_default().push(attribute.attr.range);
        }
        if !resolves_to_module(model, module_bindings, name) {
            return;
        }
        self.references.entry(name.id.as_str()).or_default().push(Reference {
            name: name.id.as_str(),
            range: name.range,
            statement: body.partition_point(|stmt| stmt.end() <= name.start()),
            evaluation: if model.in_annotation() {
                Evaluation::Annotation
            } else if function.is_some() {
                Evaluation::Deferred
            } else {
                Evaluation::Runtime
            },
            function,
        });
    }
}

pub struct Binding {
    pub range: TextRange,
    pub kind: BindingKind,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Assignment,
    Annotated,
    Augmented,
    Import,
    Function,
    Class,
    Target,
    Other,
}

#[derive(Clone, Copy)]
pub struct Reference<'a> {
    pub name: &'a str,
    pub range: TextRange,
    pub statement: usize,
    pub evaluation: Evaluation,
    pub function: Option<TextRange>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Evaluation {
    Runtime,
    Deferred,
    Annotation,
}

// `import a as b` binds its whole alias.
fn binding_range(model: &SemanticModel, binding_id: BindingId) -> TextRange {
    let binding = model.binding(binding_id);
    if let Some(Stmt::Import(import)) = binding.statement(model)
        && let Some(alias) = import.names.iter().find(|alias| alias.identifier() == binding.range)
    {
        alias.range
    } else {
        binding.range
    }
}

fn binding_kind(model: &SemanticModel, binding_id: BindingId) -> BindingKind {
    let binding = model.binding(binding_id);
    match binding.kind {
        RuffBindingKind::Import(_)
        | RuffBindingKind::FromImport(_)
        | RuffBindingKind::SubmoduleImport(_)
        | RuffBindingKind::FutureImport => BindingKind::Import,
        RuffBindingKind::FunctionDefinition(_) => BindingKind::Function,
        RuffBindingKind::ClassDefinition(_) => BindingKind::Class,
        RuffBindingKind::Annotation => BindingKind::Annotated,
        RuffBindingKind::LoopVar | RuffBindingKind::WithItemVar | RuffBindingKind::BoundException => {
            BindingKind::Target
        }
        RuffBindingKind::Assignment => match binding.statement(model) {
            Some(Stmt::Assign(_)) => BindingKind::Assignment,
            Some(Stmt::AnnAssign(_)) => BindingKind::Annotated,
            Some(Stmt::AugAssign(_)) => BindingKind::Augmented,
            Some(Stmt::Match(_)) => BindingKind::Target,
            _ => BindingKind::Other,
        },
        _ => BindingKind::Other,
    }
}

fn function_range(kind: ScopeKind) -> Option<TextRange> {
    match kind {
        ScopeKind::Function(function) => Some(function.range),
        ScopeKind::Lambda(lambda) => Some(lambda.range),
        _ => None,
    }
}

fn resolves_to_module(model: &SemanticModel, module_bindings: &FxHashSet<BindingId>, name: &ast::ExprName) -> bool {
    let id = name.id.as_str();
    if model.resolve_name(name).is_some_and(|binding_id| !module_bindings.contains(&binding_id)) {
        return false;
    }
    // Ruff resolves a load against the bindings made so far, but a name a function or comprehension binds anywhere is
    // local to all of it.
    model
        .current_scopes()
        .find(|scope| {
            matches!(scope.kind, ScopeKind::Function(_) | ScopeKind::Lambda(_) | ScopeKind::Generator { .. })
                && scope.has(id)
        })
        .is_none_or(|scope| scope.get_all(id).any(|binding_id| model.binding(binding_id).kind.is_global()))
}
