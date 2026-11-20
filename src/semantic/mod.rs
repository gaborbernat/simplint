mod append;
mod builder;
mod django;
mod runtime;
mod unused_imports;

use std::path::Path;

use itertools::Itertools;
use ruff_linter::settings::LinterSettings;
use ruff_python_ast::helpers::from_relative_import;
use ruff_python_ast::identifier::Identifier;
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

use crate::syntax::{
    annotation_name, decorator_callee, first_positional, is_classmethod, is_name, is_property_accessor, receiver,
    statement_index,
};

// Collect each Ruff check only when an enabled simplint rule needs its results.
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
    django_reports: FxHashSet<TextRange>,
    list_appends: FxHashSet<TextRange>,
    // A module can annotate lazy exports under `if TYPE_CHECKING:` and supply their values through `__getattr__`.
    type_checking_stubs: FxHashSet<TextRange>,
    // N816 checks names bound by module-level assignments, loops, with statements, assignment expressions and match
    // patterns.
    module_stores: FxHashSet<TextRange>,
    // Each comprehension with the range of its first iterable, which the enclosing scope evaluates.
    comprehensions: Vec<(TextRange, TextRange)>,
    private_imports: FxHashSet<TextRange>,
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

    pub fn is_unused_import(&self, range: TextRange) -> bool {
        self.unused_imports.contains(&range)
    }

    pub fn reported_by_dj012(&self, range: TextRange) -> bool {
        self.django_reports.contains(&range)
    }

    pub fn reported_by_plc2701(&self, range: TextRange) -> bool {
        self.private_imports.contains(&range)
    }

    pub fn is_list_append(&self, range: TextRange) -> bool {
        self.list_appends.contains(&range)
    }

    pub fn is_type_checking_stub(&self, range: TextRange) -> bool {
        self.type_checking_stubs.contains(&range)
    }

    pub fn is_module_store(&self, range: TextRange) -> bool {
        self.module_stores.contains(&range)
    }

    // A method body or nested class has its own bindings.
    pub fn class_body_bindings(&self, class: TextRange, name: &str) -> usize {
        self.model
            .scopes
            .iter()
            .find(|scope| matches!(scope.kind, ScopeKind::Class(definition) if definition.range() == class))
            .map_or(0, |scope| scope.get_all(name).count())
    }

    // Conditional imports can bind the same alias to different modules. Deletions and bare annotations supply no
    // competing value.
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

    pub fn is_builtin(&self, expr: &Expr, name: &str) -> bool {
        self.resolves_to(expr, &["", name])
    }

    // Resolve import aliases, so `mock.patch` after `from unittest import mock` matches `unittest.mock.patch`.
    pub fn resolves_to(&self, expr: &Expr, segments: &[&str]) -> bool {
        // An attribute resolves to a name ending in the attribute itself, so any other last segment rules it out.
        if let Expr::Attribute(attribute) = expr
            && segments.last() != Some(&attribute.attr.as_str())
        {
            return false;
        }
        self.model.resolve_qualified_name(expr).is_some_and(|name| name.segments() == segments)
    }

    pub const PROPERTY: &[&[&str]] = &[&["", "property"], &["functools", "cached_property"]];

    // The annotation check allows these decorators on a definition with deferred annotations.
    const KEEPS_ANNOTATIONS_LAZY: &[&[&str]] = &[
        &["", "classmethod"],
        &["", "property"],
        &["", "staticmethod"],
        &["abc", "abstractmethod"],
        &["contextlib", "asynccontextmanager"],
        &["contextlib", "contextmanager"],
        &["dataclasses", "dataclass"],
        &["functools", "cache"],
        &["functools", "cached_property"],
        &["functools", "lru_cache"],
        &["functools", "total_ordering"],
        &["functools", "wraps"],
        &["mock", "patch"],
        &["mock", "patch", "dict"],
        &["mock", "patch", "multiple"],
        &["mock", "patch", "object"],
        &["pytest", "fixture"],
        &["pytest", "hookimpl"],
        &["pytest", "hookspec"],
        &["typing", "final"],
        &["typing", "overload"],
        &["typing", "override"],
        &["typing_extensions", "final"],
        &["typing_extensions", "overload"],
        &["typing_extensions", "deprecated"],
        &["typing_extensions", "override"],
        &["unittest", "mock", "patch"],
        &["unittest", "mock", "patch", "dict"],
        &["unittest", "mock", "patch", "multiple"],
        &["unittest", "mock", "patch", "object"],
        &["warnings", "deprecated"],
    ];

    // An unknown decorator may inspect annotations as soon as the module loads.
    pub fn reads_annotations(&self, decorators: &[ast::Decorator]) -> bool {
        decorators.iter().any(|decorator| {
            !is_property_accessor(&decorator.expression)
                && !self.is_decorator(decorator, Self::KEEPS_ANNOTATIONS_LAZY)
                && !self
                    .qualified_name(decorator_callee(decorator))
                    .is_some_and(|name| name.segments().starts_with(&["pytest", "mark"]))
        })
    }

    pub fn is_decorator(&self, decorator: &ast::Decorator, names: &[&[&str]]) -> bool {
        let callee = decorator_callee(decorator);
        names.iter().any(|segments| self.resolves_to(callee, segments))
    }

    pub fn decorated_by(&self, decorators: &[ast::Decorator], names: &[&[&str]]) -> bool {
        decorators.iter().any(|decorator| self.is_decorator(decorator, names))
    }

    // Ruff uses an empty first segment for builtins.
    pub fn qualified_name<'e>(&'e self, expr: &'e Expr) -> Option<QualifiedName<'e>> {
        self.model.resolve_qualified_name(expr)
    }

    // A known module path lets Ruff qualify relative imports and local definitions. The project index supplies that
    // path separately from the semantic model.
    pub fn ruff_qualified_name<'e>(
        &'e self,
        expr: &'e Expr,
        module_path: Option<&'e [String]>,
    ) -> Option<QualifiedName<'e>> {
        let (Some(module_path), Some(name)) = (module_path, UnqualifiedName::from_expr(expr)) else {
            return self.qualified_name(expr);
        };
        let (head, tail) = name.segments().split_first().expect("a name has a segment");
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

    // B008 exempts known immutable calls and leaves mutable calls to B006. A NewType over an immutable type is also
    // exempt.
    pub fn is_b008_call(&self, func: &Expr, extend_immutable_calls: &[QualifiedName]) -> bool {
        !is_mutable_func(func, &self.model)
            && !is_immutable_func(func, &self.model, extend_immutable_calls)
            && !func
                .as_name_expr()
                .is_some_and(|name| is_immutable_newtype_call(name, &self.model, extend_immutable_calls))
    }

    pub fn has_immutable_annotation(&self, annotation: &Expr, extend_immutable_calls: &[QualifiedName]) -> bool {
        is_immutable_annotation(annotation, &self.model, extend_immutable_calls)
    }

    pub fn is_mutable_value(&self, expr: &Expr) -> bool {
        is_mutable_expr(expr, &self.model)
    }

    pub fn match_typing_expr(&self, expr: &Expr, member: &str) -> bool {
        self.model.match_typing_expr(expr, member)
    }

    // Follow the resolved binding only if it shadows no earlier binding in the same scope.
    pub fn only_binding_value(&self, name: &ast::ExprName) -> Option<&Expr> {
        let binding = self.model.binding(self.model.only_binding(name)?);
        find_binding_value(binding, &self.model)
    }

    // Ruff adds builtin bindings on lookup. Those do not count as definitions in this file.
    pub fn is_bound_in_file(&self, name: &str) -> bool {
        self.model.scopes.iter().filter_map(|scope| scope.get(name)).any(|id| !self.model.binding(id).kind.is_builtin())
    }

    pub fn class_of_binding(&self, binding: &RuffBinding<'a>) -> Option<&'a ast::StmtClassDef> {
        if !matches!(binding.kind, RuffBindingKind::ClassDefinition(_)) {
            return None;
        }
        Some(
            binding
                .statement(&self.model)
                .and_then(Stmt::as_class_def_stmt)
                .expect("a class binding's statement is its class"),
        )
    }

    // A class parameter can name a class through an annotation, a classmethod receiver or the first argument of
    // `__new__`.
    pub fn is_class_object(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Call(call) => self.is_builtin(&call.func, "type") && call.arguments.len() == 1,
            Expr::Attribute(attribute) => attribute.attr.as_str() == "__class__",
            Expr::Name(name) => {
                let bindings = self.resolved_bindings(name);
                !bindings.is_empty()
                    && bindings.iter().all(|binding| match binding.kind {
                        RuffBindingKind::ClassDefinition(_) => true,
                        RuffBindingKind::Argument => self.is_class_parameter(binding),
                        _ => false,
                    })
            }
            _ => false,
        }
    }

    fn is_class_parameter(&self, binding: &RuffBinding<'a>) -> bool {
        if let Some(annotation) = self.parameter_annotation(binding) {
            return matches!(annotation_name(annotation), Some("type" | "Type"));
        }
        let Some(Stmt::FunctionDef(function)) = binding.statement(&self.model) else {
            return false;
        };
        let receiver = function.parameters.posonlyargs.iter().chain(&function.parameters.args).next();
        receiver.is_some_and(|parameter| parameter.name().range() == binding.range)
            && (is_classmethod(function) || function.name.as_str() == "__new__")
    }

    pub fn unambiguous_parameter_annotation(&self, name: &ast::ExprName) -> Option<&'a Expr> {
        let binding = self.model.binding(self.model.resolve_name(name)?);
        let scope = &self.model.scopes[binding.scope];
        let id = scope.get_all(name.id.as_str()).exactly_one().ok()?;
        self.parameter_annotation(self.model.binding(id))
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
        self.function_binding(name).filter(|(_, global)| *global).map(|(function, _)| function)
    }

    pub fn function_of(&self, name: &ast::ExprName) -> Option<&'a ast::StmtFunctionDef> {
        self.function_binding(name).map(|(function, _)| function)
    }

    fn function_binding(&self, name: &ast::ExprName) -> Option<(&'a ast::StmtFunctionDef, bool)> {
        let bindings = self.resolved_bindings(name);
        let [binding] = bindings.as_slice() else {
            return None;
        };
        if !matches!(binding.kind, RuffBindingKind::FunctionDefinition(_)) {
            return None;
        }
        let function = binding
            .statement(&self.model)
            .and_then(Stmt::as_function_def_stmt)
            .expect("a function binding's statement is its def");
        Some((function, binding.scope.is_global()))
    }

    pub fn reads_from_import(&self, name: &ast::ExprName) -> bool {
        let bindings = self.resolved_bindings(name);
        matches!(bindings.as_slice(), [binding] if matches!(binding.kind, RuffBindingKind::FromImport(_)))
    }

    pub fn is_rebound(&self, name: &str) -> bool {
        self.bindings(name).len() > 1
    }

    // Include shadowed bindings in the scope where the name resolves.
    pub fn resolved_bindings(&self, name: &ast::ExprName) -> Vec<&RuffBinding<'a>> {
        let Some(id) = self.model.resolve_name(name) else {
            return Vec::new();
        };
        let scope = self.model.binding(id).scope;
        self.model.scopes[scope].get_all(name.id.as_str()).map(|id| self.model.binding(id)).collect()
    }

    // A loop over a literal list or tuple can give its target several known values.
    pub fn binding_values(&self, binding: &RuffBinding<'a>) -> Option<Vec<&Expr>> {
        match binding.kind {
            RuffBindingKind::LoopVar => match binding.statement(&self.model).expect("a loop variable has its loop") {
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

    pub fn is_logger_candidate(&self, func: &Expr, logger_objects: &[String]) -> bool {
        logging::is_logger_candidate(func, &self.model, logger_objects)
    }

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

    pub fn is_free_at(&self, name: &str, at: TextRange) -> bool {
        !self.is_bound(name) && !self.is_bound_around(name, at)
    }

    pub fn is_bound(&self, name: &str) -> bool {
        self.index.bindings.contains_key(name)
    }

    // Function bodies skip enclosing class namespaces. A comprehension evaluates its first iterable in the outer scope.
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

// Defaults, decorators and base classes use the enclosing scope. A parsed function or class always has a body
// statement.
fn scope_ranges(kind: &ScopeKind) -> Option<(TextRange, TextRange)> {
    match kind {
        ScopeKind::Function(function) => {
            Some((function.range(), TextRange::new(function.body[0].start(), function.range().end())))
        }
        ScopeKind::Lambda(lambda) => Some((lambda.range(), TextRange::new(lambda.body.start(), lambda.range().end()))),
        ScopeKind::Class(class) => Some((class.range(), TextRange::new(class.body[0].start(), class.range().end()))),
        _ => None,
    }
}

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

// A relative import can refer to an ordinary project module named `__future__`.
pub fn future_import(stmt: &Stmt) -> Option<&ast::StmtImportFrom> {
    stmt.as_import_from_stmt()
        .filter(|import| import.level == 0 && import.module.as_ref().is_some_and(|module| module == "__future__"))
}

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
        if let Some((receiver, class)) = enclosing_method(model)
            && let Some(attribute) = member_read(model, name, receiver, class)
        {
            let members = self.member_references.entry(class.range).or_default();
            members.entry(attribute.attr.as_str()).or_default().push(attribute.attr.range);
        }
        if !resolves_to_module(model, module_bindings, name) {
            return;
        }
        let function = model.current_scopes().filter_map(|scope| function_range(scope.kind)).last();
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

// A nested function can still use the method's receiver. Include lambdas used as methods, such as `property(lambda
// self: ...)`.
fn enclosing_method<'a>(model: &SemanticModel<'a>) -> Option<(Option<&'a str>, &'a ast::StmtClassDef)> {
    // Skip the type-parameter and implicit __class__ scopes between a method and its class.
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

// A classmethod can read an instance attribute through its receiver, as in `klass().attr`.
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
    if model.resolve_name(name).is_some_and(|binding_id| !module_bindings.contains(&binding_id)) {
        return false;
    }
    let id = name.id.as_str();
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
