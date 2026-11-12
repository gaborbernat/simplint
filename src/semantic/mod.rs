mod append;
mod builder;
mod django;
mod runtime;
mod unused_imports;

use std::path::Path;

use ruff_linter::settings::LinterSettings;
use ruff_python_ast::identifier::Identifier;

use ruff_python_ast::helpers::from_relative_import;
use ruff_python_ast::name::{QualifiedName, UnqualifiedName};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_python_semantic::analyze::logging;
use ruff_python_semantic::analyze::typing::{
    find_binding_value, is_immutable_annotation, is_immutable_func, is_immutable_newtype_call, is_mutable_expr,
    is_mutable_func,
};
use ruff_python_semantic::{
    Binding as RuffBinding, BindingId, BindingKind as RuffBindingKind, Scope, ScopeKind, SemanticModel, Symbol,
};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::syntax::{first_positional, is_name, receiver, statement_index};

// The statements of a file whose Ruff reports a rule mirrors, collected only when an enabled rule reads them.
#[derive(Clone, Copy, Default)]
#[expect(clippy::struct_excessive_bools, reason = "each flag asks for one Ruff rule's matches")]
pub struct RuffMatches {
    // The appends on a list that Ruff's FURB113 counts.
    pub list_appends: bool,
    // The statements of Django models that Ruff's DJ012 reports.
    pub django_reports: bool,
    // The import bindings that Ruff's PLC2701 reports.
    pub private_imports: bool,
    // The module-scope imports that Ruff's F401 finds unused, which SMP407 reads in an `__init__.py`.
    pub unused_imports: bool,
}

pub struct Semantic<'a> {
    model: SemanticModel<'a>,
    annotations_are_lazy: bool,
    index: Index<'a>,
    // The statements of Django models that Ruff's DJ012 reports, which the layout rules leave to it.
    django_reports: FxHashSet<TextRange>,
    // The statements that are appends on a list, which Ruff's FURB113 counts in a run of consecutive appends.
    list_appends: FxHashSet<TextRange>,
    // The annotations alone, without a value, that `if TYPE_CHECKING:` declares, which a module's `__getattr__` serves.
    type_checking_stubs: FxHashSet<TextRange>,
    // The names a module-scope store binds, which Ruff's N816 checks: an assignment, loop, `with`, walrus or `match`
    // target.
    module_stores: FxHashSet<TextRange>,
    // Each comprehension with the range of its first iterable, which the enclosing scope evaluates.
    comprehensions: Vec<(TextRange, TextRange)>,
    // The import bindings that Ruff's PLC2701 reports, by the range of the name each binds.
    private_imports: FxHashSet<TextRange>,
    // The module-scope imports Ruff's F401 finds unused, by the range of the name each binds.
    unused_imports: FxHashSet<TextRange>,
}

impl<'a> Semantic<'a> {
    pub fn new(
        path: &'a Path,
        body: &'a [Stmt],
        target_version: PythonVersion,
        settings: &'a LinterSettings,
        package: Option<&'a Path>,
        matches: RuffMatches,
    ) -> Self {
        let builder::Model {
            semantic: mut model,
            loads,
            module,
            annotations_are_lazy,
            django_reports,
            list_appends,
            type_checking_stubs,
            module_stores,
            comprehensions,
            private_imports,
        } = builder::build(path, body, target_version, settings, package, matches);
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
        let unused_imports = if matches.unused_imports {
            unused_imports::unused_imports(&model, body, &settings.pyflakes.extend_generics)
        } else {
            FxHashSet::default()
        };
        Self {
            model,
            annotations_are_lazy,
            index,
            django_reports,
            list_appends,
            type_checking_stubs,
            module_stores,
            comprehensions,
            private_imports,
            unused_imports,
        }
    }

    // Whether Ruff's F401 finds the import binding its name at `range` unused.
    pub fn is_unused_import(&self, range: TextRange) -> bool {
        self.unused_imports.contains(&range)
    }

    // Whether Ruff's DJ012 reports the statement at `range`, a statement of a Django model's body; only answered when
    // `RuffMatches::django_reports` asked for it.
    pub fn reported_by_dj012(&self, range: TextRange) -> bool {
        self.django_reports.contains(&range)
    }

    // Whether Ruff's PLC2701 reports the import binding at `range`, the name the import binds; only answered when
    // `RuffMatches::private_imports` asked for it.
    pub fn reported_by_plc2701(&self, range: TextRange) -> bool {
        self.private_imports.contains(&range)
    }

    // Whether the statement at `range` is an append on a list, as FURB113 matches it; only answered when
    // `RuffMatches::list_appends` asked for it.
    pub fn is_list_append(&self, range: TextRange) -> bool {
        self.list_appends.contains(&range)
    }

    // Whether the binding at `range` is an annotation without a value under `if TYPE_CHECKING:`, which only types a
    // name.
    pub fn is_type_checking_stub(&self, range: TextRange) -> bool {
        self.type_checking_stubs.contains(&range)
    }

    // Whether the name at `range` is stored in the module scope, where Ruff's N816 reads it.
    pub fn is_module_store(&self, range: TextRange) -> bool {
        self.module_stores.contains(&range)
    }

    // How many bindings of `name` the class body at `class` makes in the body's own scope; a method's locals and nested
    // classes or comprehensions have scopes of their own.
    pub fn class_body_bindings(&self, class: TextRange, name: &str) -> usize {
        self.model
            .scopes
            .iter()
            .find(|scope| matches!(scope.kind, ScopeKind::Class(definition) if definition.range() == class))
            .map_or(0, |scope| scope.get_all(name).count())
    }

    // Whether every binding of the name `value` reads in its scope imports `module`, so a name also bound to another
    // module, as in `try: import regex as re` beside `import re`, does not read as the module. A `del` or a bare
    // annotation binds nothing, so it does not count.
    pub fn is_only_import_of(&self, value: &Expr, module: &[&str]) -> bool {
        value.as_name_expr().is_some_and(|name| {
            self.resolved_bindings(name)
                .iter()
                .filter(|binding| !matches!(binding.kind, RuffBindingKind::Deletion | RuffBindingKind::Annotation))
                .all(|binding| {
                    matches!(
                        &binding.kind,
                        RuffBindingKind::Import(import) if import.qualified_name.segments() == module
                    )
                })
        })
    }

    // Whether `expr` is the builtin `name`, not something the file binds the name to in the scope it is read in.
    pub fn is_builtin(&self, expr: &Expr, name: &str) -> bool {
        self.resolves_to(expr, &["", name])
    }

    // Whether `expr` names `segments`, resolved as Ruff does through the imports that bind its head, so `mock.patch`
    // after `from unittest import mock` names `unittest.mock.patch`.
    pub fn resolves_to(&self, expr: &Expr, segments: &[&str]) -> bool {
        // An attribute resolves to a name ending in the attribute itself, so any other last segment rules it out.
        if let Expr::Attribute(attribute) = expr
            && segments.last() != Some(&attribute.attr.as_str())
        {
            return false;
        }
        self.model.resolve_qualified_name(expr).is_some_and(|name| name.segments() == segments)
    }

    // What `expr` names, resolved as `resolves_to` does; a builtin's first segment is empty.
    pub fn qualified_name<'e>(&'e self, expr: &'e Expr) -> Option<QualifiedName<'e>> {
        self.model.resolve_qualified_name(expr)
    }

    // The name Ruff gives `expr` in a module it knows the dotted path of: there a relative import and a definition in
    // the module resolve too. The model stays without that path, as the rules read those names through the project.
    pub fn ruff_qualified_name<'e>(
        &'e self,
        expr: &'e Expr,
        module_path: Option<&'e [String]>,
    ) -> Option<QualifiedName<'e>> {
        let (Some(module_path), Some(name)) = (module_path, UnqualifiedName::from_expr(expr)) else {
            return self.qualified_name(expr);
        };
        let (head, tail) = name.segments().split_first()?;
        let Symbol::Binding(id) = self.model.lookup_symbol(head) else {
            return self.qualified_name(expr);
        };
        match &self.model.binding(id).kind {
            RuffBindingKind::FromImport(import) if import.qualified_name.is_unresolved_import() => {
                from_relative_import(module_path, import.qualified_name.segments(), tail)
            }
            RuffBindingKind::ClassDefinition(_) | RuffBindingKind::FunctionDefinition(_) => {
                Some(module_path.iter().map(String::as_str).chain(name.segments().iter().copied()).collect())
            }
            _ => self.qualified_name(expr),
        }
    }

    // Whether Ruff's B008 reports a call to `func` in a parameter default. It leaves alone a mutable or immutable
    // callee, and a `NewType` of an immutable type.
    pub fn is_b008_call(&self, func: &Expr, extend_immutable_calls: &[QualifiedName]) -> bool {
        !is_mutable_func(func, &self.model)
            && !is_immutable_func(func, &self.model, extend_immutable_calls)
            && !func
                .as_name_expr()
                .is_some_and(|name| is_immutable_newtype_call(name, &self.model, extend_immutable_calls))
    }

    // Whether Ruff's B008 skips a parameter with this annotation, as the annotation names an immutable type.
    pub fn has_immutable_annotation(&self, annotation: &Expr, extend_immutable_calls: &[QualifiedName]) -> bool {
        is_immutable_annotation(annotation, &self.model, extend_immutable_calls)
    }

    // Whether `expr` is a mutable display, comprehension, or a call of a mutable builtin or collection.
    pub fn is_mutable_value(&self, expr: &Expr) -> bool {
        is_mutable_expr(expr, &self.model)
    }

    // Whether `expr` names `typing.{member}`, or its `typing_extensions` backport.
    pub fn match_typing_expr(&self, expr: &Expr, member: &str) -> bool {
        self.model.match_typing_expr(expr, member)
    }

    // The value a load of `name` reads, when it resolves to the first binding of the name in its scope, as Ruff's rules
    // that follow a variable to its literal resolve it.
    pub fn only_binding_value(&self, name: &ast::ExprName) -> Option<&Expr> {
        let binding = self.model.binding(self.model.only_binding(name)?);
        find_binding_value(binding, &self.model)
    }

    // Whether a scope of the file binds the name, which shadows a builtin of that name. Ruff adds a builtin's binding
    // on its first lookup, which is not a binding of the file.
    pub fn is_bound_in_file(&self, name: &str) -> bool {
        self.model.scopes.iter().filter_map(|scope| scope.get(name)).any(|id| !self.model.binding(id).kind.is_builtin())
    }

    pub fn class_of_binding(&self, binding: &RuffBinding<'a>) -> Option<&'a ast::StmtClassDef> {
        if !matches!(binding.kind, RuffBindingKind::ClassDefinition(_)) {
            return None;
        }
        binding.statement(&self.model)?.as_class_def_stmt()
    }

    pub fn parameter_annotation(&self, binding: &RuffBinding<'a>) -> Option<&'a Expr> {
        let Stmt::FunctionDef(function) = binding.statement(&self.model)? else {
            return None;
        };
        function.parameters.iter().find(|parameter| parameter.name().range() == binding.range)?.annotation()
    }

    pub fn module_function(&self, name: &ast::ExprName) -> Option<&'a ast::StmtFunctionDef> {
        if self.is_rebound(name.id.as_str()) {
            return None;
        }
        let bindings = self.resolved_bindings(name);
        let [binding] = bindings.as_slice() else {
            return None;
        };
        if !matches!(binding.kind, RuffBindingKind::FunctionDefinition(_)) || !binding.scope.is_global() {
            return None;
        }
        binding.statement(&self.model)?.as_function_def_stmt()
    }

    // The `def` a load of `name` reads, when its scope binds the name once, and to that `def`.
    pub fn function_of(&self, name: &ast::ExprName) -> Option<&'a ast::StmtFunctionDef> {
        let bindings = self.resolved_bindings(name);
        let [binding] = bindings.as_slice() else {
            return None;
        };
        if !matches!(binding.kind, RuffBindingKind::FunctionDefinition(_)) {
            return None;
        }
        binding.statement(&self.model)?.as_function_def_stmt()
    }

    // Whether a load of `name` reads a `from ... import`, the only binding of the name in its scope.
    pub fn reads_from_import(&self, name: &ast::ExprName) -> bool {
        let bindings = self.resolved_bindings(name);
        matches!(bindings.as_slice(), [binding] if matches!(binding.kind, RuffBindingKind::FromImport(_)))
    }

    pub fn is_rebound(&self, name: &str) -> bool {
        self.bindings(name).len() > 1
    }

    // Every binding of `name` in the scope a load of it resolves to, the shadowed ones too; empty when none resolves.
    pub fn resolved_bindings(&self, name: &ast::ExprName) -> Vec<&RuffBinding<'a>> {
        let Some(id) = self.model.resolve_name(name) else {
            return Vec::new();
        };
        let scope = self.model.binding(id).scope;
        self.model.scopes[scope].get_all(name.id.as_str()).map(|id| self.model.binding(id)).collect()
    }

    // The values a binding gives its name, as far as the rules read them: the value an assignment gives, or each item
    // of a loop over a literal list or tuple. `None` when no value is known, as for an import or a function parameter.
    pub fn binding_values(&self, binding: &RuffBinding<'a>) -> Option<Vec<&Expr>> {
        match binding.kind {
            RuffBindingKind::LoopVar => match binding.statement(&self.model)? {
                Stmt::For(ast::StmtFor { target, iter, .. })
                    if target.as_name_expr().is_some_and(|target| target.range() == binding.range) =>
                {
                    match iter.as_ref() {
                        Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })
                            if !elts.iter().any(Expr::is_starred_expr) =>
                        {
                            Some(elts.iter().collect())
                        }
                        _ => None,
                    }
                }
                _ => None,
            },
            _ => find_binding_value(binding, &self.model).map(|value| vec![value]),
        }
    }

    // Whether `func` is a method of a logger that Ruff's logging rules check: the `logging` module, a logger it
    // creates, or one of the `logger-objects`.
    pub fn is_logger_candidate(&self, func: &Expr, logger_objects: &[String]) -> bool {
        logging::is_logger_candidate(func, &self.model, logger_objects)
    }

    // Whether annotations wait until something reads them: from `from __future__ import annotations` or a target
    // version that defers them.
    pub fn annotations_are_lazy(&self) -> bool {
        self.annotations_are_lazy
    }

    pub fn all_bindings(&self) -> impl Iterator<Item = (&'a str, &[Binding])> {
        self.index.bindings.iter().map(|(name, bindings)| (*name, bindings.as_slice()))
    }

    pub fn references(&self) -> impl Iterator<Item = &Reference<'a>> {
        self.index.references.values().flatten()
    }

    pub fn member_references(&self, class: TextRange, name: &str) -> &[TextRange] {
        self.index.member_references.get(&class).and_then(|members| members.get(name)).map_or(&[], Vec::as_slice)
    }

    pub fn local_bindings(&self, name: &str) -> &[TextRange] {
        self.index.local_bindings.get(name).map_or(&[], Vec::as_slice)
    }

    // Whether `name` is bound nowhere a read at `at` could see, so a builtin of that name is what `at` reads.
    pub fn is_free_at(&self, name: &str, at: TextRange) -> bool {
        !self.is_bound(name) && !self.is_bound_around(name, at)
    }

    pub fn is_bound(&self, name: &str) -> bool {
        self.index.bindings.contains_key(name)
    }

    // Whether a binding of `name` made directly in a scope that encloses `at` shadows the name there: a function or
    // lambda body, a class body that no function inside it encloses, or a comprehension past its first iterable, which
    // the enclosing scope evaluates.
    pub fn is_bound_around(&self, name: &str, at: TextRange) -> bool {
        let scopes = || self.model.scopes.iter().filter_map(|scope| Some((scope, scope_ranges(&scope.kind)?)));
        let nested_in_class = |class: TextRange| {
            scopes().any(|(scope, (node, body))| {
                (scope.kind.is_function() || scope.kind.is_lambda())
                    && body.contains_range(at)
                    && class.contains_range(node)
            })
        };
        let bound_directly = |scope: &Scope| scope.get_all(name).any(|id| !self.model.binding(id).kind.is_global());
        scopes().any(|(scope, (node, body))| {
            body.contains_range(at) && bound_directly(scope) && (!scope.kind.is_class() || !nested_in_class(node))
        }) || self.model.scopes.iter().filter(|scope| scope.kind.is_generator()).any(|scope| {
            scope.get_all(name).any(|id| {
                let range = self.model.binding(id).range;
                self.comprehensions
                    .iter()
                    .filter(|(comprehension, _)| comprehension.contains_range(range))
                    .min_by_key(|(comprehension, _)| comprehension.len())
                    .is_some_and(|(comprehension, first)| comprehension.contains_range(at) && !first.contains_range(at))
            })
        })
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

    pub fn bindings(&self, name: &str) -> &[Binding] {
        self.index.bindings.get(name).map_or(&[], Vec::as_slice)
    }

    // In source order.
    pub fn references_to(&self, name: &str) -> &[Reference<'a>] {
        self.index.references.get(name).map_or(&[], Vec::as_slice)
    }
}

// The body is the part of a scope's node after its header, since a header's defaults, decorators and bases run in the
// enclosing scope.
fn scope_ranges(kind: &ScopeKind) -> Option<(TextRange, TextRange)> {
    match kind {
        ScopeKind::Function(function) => {
            Some((function.range(), TextRange::new(function.body.first()?.start(), function.range().end())))
        }
        ScopeKind::Lambda(lambda) => Some((lambda.range(), TextRange::new(lambda.body.start(), lambda.range().end()))),
        ScopeKind::Class(class) => {
            Some((class.range(), TextRange::new(class.body.first()?.start(), class.range().end())))
        }
        _ => None,
    }
}

// A `from __future__` import statement; a relative import of a `__future__` module of the package is not one.
// What a binding names, as the rules that follow a name into the project read it.
pub fn binding_referent(binding: &RuffBinding<'_>) -> Referent {
    match binding.kind {
        RuffBindingKind::FunctionDefinition(_) => Referent::Function,
        RuffBindingKind::ClassDefinition(_) => Referent::Class,
        RuffBindingKind::Import(_) | RuffBindingKind::SubmoduleImport(_) => Referent::Module,
        RuffBindingKind::FromImport(_) => Referent::Member,
        RuffBindingKind::Builtin => Referent::Builtin,
        RuffBindingKind::Assignment | RuffBindingKind::NamedExprAssignment => Referent::Value,
        _ => Referent::Other,
    }
}

pub fn future_import(stmt: &Stmt) -> Option<&ast::StmtImportFrom> {
    stmt.as_import_from_stmt()
        .filter(|import| import.level == 0 && import.module.as_ref().is_some_and(|module| module == "__future__"))
}

// The module-level names of `Semantic`, by name, so rules look them up instead of scanning.
#[derive(Default)]
struct Index<'a> {
    bindings: FxHashMap<&'a str, Vec<Binding>>,
    references: FxHashMap<&'a str, Vec<Reference<'a>>>,
    member_references: FxHashMap<TextRange, FxHashMap<&'a str, Vec<TextRange>>>,
    global_declarations: FxHashMap<&'a str, Vec<TextRange>>,
    local_bindings: FxHashMap<&'a str, Vec<TextRange>>,
}

impl<'a> Index<'a> {
    fn collect_bindings(&mut self, model: &SemanticModel<'a>) -> FxHashSet<BindingId> {
        let mut module_bindings = FxHashSet::default();
        for scope in model.scopes.iter() {
            for (name, latest) in scope.bindings() {
                let chain: Vec<BindingId> = scope.shadowed_bindings(latest).collect();
                if !scope.kind.is_module() {
                    let locals = chain
                        .iter()
                        .filter(|binding_id| !model.binding(**binding_id).kind.is_global())
                        .map(|binding_id| binding_range(model, *binding_id));
                    self.local_bindings.entry(name).or_default().extend(locals);
                }
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
                        module_scope: model.binding(*binding_id).scope.is_global(),
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
        if let Some((receiver, class)) = enclosing_method(model)
            && let Some(attribute) = member_read(model, name, receiver, class)
        {
            let members = self.member_references.entry(class.range).or_default();
            members.entry(attribute.attr.as_str()).or_default().push(attribute.attr.range);
        }
        if !resolves_to_module(model, module_bindings, name) {
            return;
        }
        // PEP 695 evaluates type parameter bounds and defaults, and `type` alias values, lazily.
        let lazy = model.in_deferred_type_definition() || model.in_deferred_type_alias_value();
        self.references.entry(name.id.as_str()).or_default().push(Reference {
            name: name.id.as_str(),
            range: name.range,
            statement: statement_index(body, name.start()),
            evaluation: if model.in_annotation() {
                Evaluation::Annotation
            } else if function.is_some() || lazy {
                Evaluation::Deferred
            } else {
                Evaluation::Runtime
            },
            function,
        });
    }
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
            Some(Stmt::AnnAssign(_)) => BindingKind::Annotated,
            Some(Stmt::AugAssign(_)) => BindingKind::Augmented,
            Some(Stmt::Match(_)) => BindingKind::Target,
            Some(Stmt::TypeAlias(_)) => BindingKind::TypeAlias,
            _ => BindingKind::Assignment,
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

// The receiver of the method a load runs in, and its class. A function nested in the method runs in it too, so the
// innermost function directly in a class body is the one whose receiver the load reads. A lambda in a class body, as
// `property(lambda self: ...)`, is a method too.
fn enclosing_method<'a>(model: &SemanticModel<'a>) -> Option<(Option<&'a str>, &'a ast::StmtClassDef)> {
    // Ruff's scopes between a method and its class, such as the type parameter and `__class__` cell scopes, do not
    // count.
    let mut method = None;
    for scope in model.current_scopes() {
        match scope.kind {
            ScopeKind::Function(function) => method = Some(receiver(function)),
            ScopeKind::Lambda(lambda) => method = Some(lambda.parameters.as_deref().and_then(first_positional)),
            ScopeKind::Class(class) => {
                if let Some(receiver) = method.take() {
                    return Some((receiver, class));
                }
            }
            _ => {}
        }
    }
    None
}

// The attribute a load of `name` reads off an instance or the class inside a method of `class` with `receiver`:
// `name.attr` where `name` is the receiver, the class or a local the method assigns from a call of the receiver, and
// `name().attr` where `name` is called, as `klass().attr` for the receiver `klass`.
fn member_read<'a>(
    model: &SemanticModel<'a>,
    name: &ast::ExprName,
    receiver: Option<&str>,
    class: &ast::StmtClassDef,
) -> Option<&'a ast::ExprAttribute> {
    let is_receiver = receiver == Some(name.id.as_str());
    match model.current_expression_parent()? {
        Expr::Attribute(attribute)
            if is_receiver || name.id == class.name.as_str() || is_receiver_instance(model, name, receiver) =>
        {
            Some(attribute)
        }
        Expr::Call(call) if is_receiver && call.func.range() == name.range => {
            match model.current_expression_grandparent()? {
                Expr::Attribute(attribute) => Some(attribute),
                _ => None,
            }
        }
        _ => None,
    }
}

fn is_receiver_instance(model: &SemanticModel, name: &ast::ExprName, receiver: Option<&str>) -> bool {
    let (Some(receiver), Some(binding)) = (receiver, model.resolve_name(name)) else {
        return false;
    };
    let Some(Stmt::Assign(ast::StmtAssign { value, .. })) = model.binding(binding).statement(model) else {
        return false;
    };
    value.as_call_expr().is_some_and(|call| {
        is_name(&call.func, receiver)
            || call.func.as_attribute_expr().is_some_and(|attribute| attribute.attr.as_str() == "__new__")
                && call.arguments.args.first().is_some_and(|first| is_name(first, receiver))
    })
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

pub struct Binding {
    pub range: TextRange,
    pub kind: BindingKind,
    // Whether the binding is made in the module scope, rather than in a function's `global` assignment.
    pub module_scope: bool,
}

// What a name is bound to: a function, a class, a module, a name imported from a module, a builtin, or a value that an
// assignment gives, which the rules follow to its value.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Referent {
    Function,
    Class,
    Module,
    Member,
    Builtin,
    Value,
    Other,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Assignment,
    Annotated,
    Augmented,
    Import,
    Function,
    Class,
    TypeAlias,
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
