// Ported from the parts of Ruff's `Checker` (crates/ruff_linter/src/checkers/ast/mod.rs, MIT) that build its
// `SemanticModel`. Rule dispatch, docstrings, branches, type definitions (string annotations, `typing.cast` and the
// like), `__all__` exports and `global` pre-scanning are left out, and type parameter bounds are visited in place
// instead of deferred.
use std::path::Path;

use ruff_linter::rules::flake8_type_checking::settings::RuntimeSemantics;
use ruff_linter::settings::LinterSettings;
use ruff_python_ast::helpers::{collect_import_from_member, is_dunder};
use ruff_python_ast::identifier::Identifier;
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{
    self as ast, AnyParameterRef, Comprehension, ExceptHandler, Expr, ExprContext, Parameters, Pattern, PySourceType,
    PythonVersion, Stmt,
};
use ruff_python_semantic::analyze::typing::is_type_checking_block;
use ruff_python_semantic::{
    Binding, BindingFlags, BindingKind, FromImport, GeneratorKind, Import, Imported, Module, ModuleKind, ModuleSource,
    ResolvedReference, ScopeId, ScopeKind, SemanticModel, SemanticModelFlags, Snapshot, StarImport, SubmoduleImport,
};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use super::RuffMatches;
use super::append::is_list_append;
use super::django::unordered_model_statements;
use super::runtime::Runtime;
use crate::semantic::future_import;
use crate::syntax::{pattern_capture, signature_annotations};

pub struct Model<'a> {
    pub semantic: SemanticModel<'a>,
    pub loads: Vec<(&'a ast::ExprName, Snapshot)>,
    // The model at module level once every top-level statement ran.
    pub module: Snapshot,
    pub annotations_are_lazy: bool,
    pub django_reports: FxHashSet<TextRange>,
    pub list_appends: FxHashSet<TextRange>,
    pub type_checking_stubs: FxHashSet<TextRange>,
    pub comprehensions: Vec<(TextRange, TextRange)>,
    pub private_imports: FxHashSet<TextRange>,
}

pub fn build<'a>(
    path: &'a Path,
    body: &'a [Stmt],
    target_version: PythonVersion,
    settings: &'a LinterSettings,
    package: Option<&'a Path>,
    matches: RuffMatches,
) -> Model<'a> {
    let module = Module {
        kind: if path.ends_with("__init__.py") { ModuleKind::Package } else { ModuleKind::Module },
        source: ModuleSource::File(path),
        python_ast: body,
        name: path.file_stem().and_then(|stem| stem.to_str()),
    };
    let mut builder = Builder {
        semantic: SemanticModel::new(&[], &[], target_version, PySourceType::Python, path, module),
        target_version,
        runtime: Runtime::new(settings),
        matches,
        functions: Vec::new(),
        lambdas: Vec::new(),
        loads: Vec::new(),
        django_reports: FxHashSet::default(),
        list_appends: FxHashSet::default(),
        type_checking_stubs: FxHashSet::default(),
        comprehensions: Vec::new(),
    };
    builder.visit_body(body);
    let module = builder.semantic.snapshot();
    let annotations_are_lazy = builder.annotations_are_lazy();
    builder.visit_deferred();
    let private_imports = if builder.matches.private_imports {
        private_import_reports(&builder.semantic, package)
    } else {
        FxHashSet::default()
    };
    Model {
        semantic: builder.semantic,
        loads: builder.loads,
        module,
        annotations_are_lazy,
        django_reports: builder.django_reports,
        list_appends: builder.list_appends,
        type_checking_stubs: builder.type_checking_stubs,
        comprehensions: builder.comprehensions,
        private_imports,
    }
}

// Ported from Ruff's PLC2701 (crates/ruff_linter/src/rules/pylint/rules/import_private_name.rs, MIT), which reads the
// bindings of each module and function scope once their references resolve. Keyed by the range of the name a binding
// makes, which is the alias of its import.
fn private_import_reports(semantic: &SemanticModel, package: Option<&Path>) -> FxHashSet<TextRange> {
    semantic
        .scopes
        .iter()
        .filter(|scope| matches!(scope.kind, ScopeKind::Function(_) | ScopeKind::Module))
        .flat_map(ruff_python_semantic::Scope::binding_ids)
        .map(|binding_id| semantic.binding(binding_id))
        .filter(|binding| is_private_import(semantic, binding, package))
        .map(|binding| binding.range)
        .collect()
}

fn is_private_import(semantic: &SemanticModel, binding: &Binding, package: Option<&Path>) -> bool {
    let Some(import) = binding.as_any_import().filter(|import| !import.is_submodule_import()) else {
        return false;
    };
    if import.module_name().starts_with(&["."]) {
        return false;
    }
    let segments = import.qualified_name().segments();
    let root_module = segments[0];
    let member_name = import.member_name();
    if is_dunder(root_module) || is_dunder(&member_name) {
        return false;
    }
    if package.is_some_and(|path| path.ends_with(root_module)) {
        return false;
    }
    if !segments.iter().any(|name| name.starts_with('_') && !is_dunder(name)) || matches!(segments, ["os", "_exit"]) {
        return false;
    }
    binding.references().next().is_none() || binding.references().any(|id| !is_typing_only(semantic.reference(id)))
}

fn is_typing_only(reference: &ResolvedReference) -> bool {
    reference.in_typing_context() || reference.in_runtime_evaluated_annotation()
}

struct Builder<'a> {
    semantic: SemanticModel<'a>,
    target_version: PythonVersion,
    runtime: Runtime<'a>,
    matches: RuffMatches,
    functions: Vec<(&'a ast::StmtFunctionDef, Snapshot)>,
    lambdas: Vec<(&'a ast::ExprLambda, Snapshot)>,
    loads: Vec<(&'a ast::ExprName, Snapshot)>,
    django_reports: FxHashSet<TextRange>,
    list_appends: FxHashSet<TextRange>,
    type_checking_stubs: FxHashSet<TextRange>,
    comprehensions: Vec<(TextRange, TextRange)>,
}

impl Builder<'_> {
    // The appends on a list that Ruff's FURB113 matches, recorded where the scope reads as Ruff reads it.
    fn note_list_append(&mut self, stmt: &Stmt) {
        if self.matches.list_appends && is_list_append(&self.semantic, stmt) {
            self.list_appends.insert(stmt.range());
        }
    }
}

impl<'a> Visitor<'a> for Builder<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        self.semantic.push_node(stmt);
        self.note_list_append(stmt);
        if let Some(import) = future_import(stmt)
            && !import.is_lazy
            && import.names.iter().any(|alias| alias.name.as_str() == "annotations")
        {
            self.semantic.flags |= SemanticModelFlags::FUTURE_ANNOTATIONS;
        }
        let flags = self.semantic.flags;
        match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) => {
                for alias in names {
                    self.semantic.add_module(alias.name.split_once('.').map_or(alias.name.as_str(), |(top, _)| top));
                    let qualified_name = Box::new(QualifiedName::user_defined(&alias.name));
                    match (&alias.asname, alias.name.split_once('.')) {
                        (None, Some((module, _))) => {
                            let kind = BindingKind::SubmoduleImport(SubmoduleImport { qualified_name });
                            self.add_binding(module, alias.identifier(), kind);
                        }
                        (asname, _) => {
                            let name = asname.as_ref().unwrap_or(&alias.name);
                            self.add_binding(name, alias.identifier(), BindingKind::Import(Import { qualified_name }));
                        }
                    }
                }
            }
            Stmt::ImportFrom(ast::StmtImportFrom { names, module, level, is_lazy, .. }) => {
                let module = module.as_deref();
                if *level == 0
                    && let Some(module) = module
                {
                    self.semantic.add_module(module.split_once('.').map_or(module, |(top, _)| top));
                }
                for alias in names {
                    let name = alias.asname.as_ref().unwrap_or(&alias.name);
                    if !is_lazy && module == Some("__future__") {
                        self.add_binding(name, alias.identifier(), BindingKind::FutureImport);
                    } else if &alias.name == "*" {
                        self.semantic.current_scope_mut().add_star_import(StarImport { level: *level, module });
                    } else {
                        let qualified_name = Box::new(collect_import_from_member(*level, module, &alias.name));
                        self.add_binding(
                            name,
                            alias.identifier(),
                            BindingKind::FromImport(FromImport { qualified_name }),
                        );
                    }
                }
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) if !self.semantic.scope_id.is_global() => {
                for name in names {
                    let global = self.semantic.global_binding(name);
                    self.declare(name, name.range(), BindingKind::Global(global), BindingFlags::GLOBAL);
                }
            }
            Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) if !self.semantic.scope_id.is_global() => {
                for name in names {
                    if let Some((scope_id, binding_id)) = self.semantic.nonlocal(name) {
                        let kind = BindingKind::Nonlocal(binding_id, scope_id);
                        self.declare(name, name.range(), kind, BindingFlags::NONLOCAL);
                    }
                }
            }
            _ => {}
        }
        match stmt {
            Stmt::FunctionDef(function_def) => self.visit_function_def(stmt, function_def),
            Stmt::ClassDef(class_def) => {
                for decorator in &class_def.decorator_list {
                    self.visit_decorator(decorator);
                }
                self.semantic.push_scope(ScopeKind::Type);
                if let Some(type_params) = &class_def.type_params {
                    self.visit_type_params(type_params);
                }
                if let Some(arguments) = &class_def.arguments {
                    self.visit_arguments(arguments);
                }
                self.semantic.push_scope(ScopeKind::Class(class_def));
                self.visit_body(&class_def.body);
                if self.matches.django_reports {
                    self.django_reports.extend(unordered_model_statements(&self.semantic, class_def));
                }
                let scope_id = self.semantic.scope_id;
                self.semantic.pop_scope();
                self.semantic.pop_scope();
                self.add_binding(&class_def.name, stmt.identifier(), BindingKind::ClassDefinition(scope_id));
            }
            Stmt::TypeAlias(type_alias) => self.visit_type_alias(type_alias),
            Stmt::If(stmt_if) if is_type_checking_block(stmt_if, &self.semantic) => {
                self.visit_type_checking_if(stmt_if);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
        self.semantic.flags = flags;
        self.semantic.pop_node();
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        let runtime = match self.semantic.current_scope().kind {
            ScopeKind::Class(class_def) => self.runtime.class(class_def, &self.semantic),
            _ => RuntimeSemantics::Default,
        };
        self.visit_annotation_as(expr, runtime);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        self.semantic.push_node(expr);
        let flags = self.semantic.flags;
        match expr {
            Expr::Name(name) => match name.ctx {
                ExprContext::Load => {
                    self.semantic.resolve_load(name);
                    self.loads.push((name, self.semantic.snapshot()));
                }
                ExprContext::Store => self.handle_node_store(name),
                // Only a tree with syntax errors holds `Invalid`, and the rules never see one.
                ExprContext::Del | ExprContext::Invalid => self.handle_node_delete(name),
            },
            Expr::ListComp(ast::ExprListComp { elt, generators, .. }) => {
                self.visit_generators(GeneratorKind::ListComprehension, expr.range(), generators);
                self.visit_expr(elt);
            }
            Expr::SetComp(ast::ExprSetComp { elt, generators, .. }) => {
                self.visit_generators(GeneratorKind::SetComprehension, expr.range(), generators);
                self.visit_expr(elt);
            }
            Expr::Generator(ast::ExprGenerator { elt, generators, .. }) => {
                self.visit_generators(GeneratorKind::Generator, expr.range(), generators);
                self.visit_expr(elt);
            }
            Expr::DictComp(ast::ExprDictComp { key, value, generators, .. }) => {
                self.visit_generators(GeneratorKind::DictComprehension, expr.range(), generators);
                if let Some(key) = key {
                    self.visit_expr(key);
                }
                self.visit_expr(value);
            }
            Expr::Lambda(lambda) => {
                for default in lambda
                    .parameters
                    .iter()
                    .flat_map(|parameters| parameters.iter_non_variadic_params())
                    .filter_map(|parameter| parameter.default.as_deref())
                {
                    self.visit_expr(default);
                }
                self.semantic.push_scope(ScopeKind::Lambda(lambda));
                self.lambdas.push((lambda, self.semantic.snapshot()));
            }
            Expr::Named(ast::ExprNamed { target, value, .. }) => {
                self.visit_expr(value);
                self.semantic.flags |= SemanticModelFlags::NAMED_EXPRESSION_ASSIGNMENT;
                self.visit_expr(target);
            }
            _ => visitor::walk_expr(self, expr),
        }
        if matches!(
            expr,
            Expr::Lambda(_) | Expr::Generator(_) | Expr::ListComp(_) | Expr::DictComp(_) | Expr::SetComp(_)
        ) {
            self.semantic.pop_scope();
        }
        self.semantic.flags = flags;
        self.semantic.pop_node();
    }

    fn visit_except_handler(&mut self, except_handler: &'a ExceptHandler) {
        let ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name, .. }) = except_handler;
        let bound = name.as_ref().map(|name| {
            let shadowed = self.semantic.lookup_binding(name);
            self.add_binding(name, name.range(), BindingKind::BoundException);
            (name, shadowed)
        });
        visitor::walk_except_handler(self, except_handler);
        if let Some((name, shadowed)) = bound {
            self.add_binding(name, name.range(), BindingKind::UnboundException(shadowed));
        }
    }

    fn visit_parameters(&mut self, parameters: &'a Parameters) {
        for parameter in parameters.iter().map(AnyParameterRef::as_parameter) {
            self.add_binding(&parameter.name, parameter.identifier(), BindingKind::Argument);
        }
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        if let Some(name) = pattern_capture(pattern) {
            self.add_binding(name, name.range(), BindingKind::Assignment);
        }
        visitor::walk_pattern(self, pattern);
    }

    fn visit_type_param(&mut self, type_param: &'a ast::TypeParam) {
        let name = type_param.name();
        self.add_binding(name, name.range(), BindingKind::TypeParam);
        let flags = self.semantic.flags;
        self.semantic.flags |= SemanticModelFlags::TYPE_PARAM_DEFINITION;
        visitor::walk_type_param(self, type_param);
        self.semantic.flags = flags;
    }
}

impl<'a> Builder<'a> {
    fn visit_type_alias(&mut self, type_alias: &'a ast::StmtTypeAlias) {
        let ast::StmtTypeAlias { name, type_params, value, .. } = type_alias;
        self.semantic.push_scope(ScopeKind::Type);
        self.semantic.flags |= SemanticModelFlags::DEFERRED_TYPE_ALIAS;
        if let Some(type_params) = type_params {
            self.visit_type_params(type_params);
        }
        let flags = self.semantic.flags;
        self.semantic.flags |= SemanticModelFlags::TYPE_PARAM_DEFINITION;
        self.visit_expr(value);
        self.semantic.flags = flags;
        self.semantic.pop_scope();
        self.visit_expr(name);
    }

    fn visit_type_checking_if(&mut self, stmt_if: &'a ast::StmtIf) {
        self.visit_expr(&stmt_if.test);
        let flags = self.semantic.flags;
        self.semantic.flags |= SemanticModelFlags::TYPE_CHECKING_BLOCK;
        self.visit_body(&stmt_if.body);
        self.semantic.flags = flags;
        for clause in &stmt_if.elif_else_clauses {
            self.visit_elif_else_clause(clause);
        }
    }

    // Python evaluates an annotation when it runs the statement, except inside a function body or when annotations are
    // lazy. A class or decorator that Ruff's settings list as runtime-required or ambiguous makes it so regardless.
    fn visit_annotation_as(&mut self, expr: &'a Expr, runtime: RuntimeSemantics) {
        let flags = self.semantic.flags;
        self.semantic.flags |= match runtime {
            RuntimeSemantics::Required => SemanticModelFlags::RUNTIME_REQUIRED_ANNOTATION,
            RuntimeSemantics::Ambiguous => SemanticModelFlags::RUNTIME_AMBIGUOUS_ANNOTATION,
            RuntimeSemantics::Default
                if self.annotations_are_lazy() || self.semantic.current_scope().kind.is_function() =>
            {
                SemanticModelFlags::TYPING_ONLY_ANNOTATION
            }
            RuntimeSemantics::Default => SemanticModelFlags::RUNTIME_EVALUATED_ANNOTATION,
        };
        self.visit_expr(expr);
        self.semantic.flags = flags;
    }

    fn annotations_are_lazy(&self) -> bool {
        self.semantic.future_annotations_or_stub() || self.target_version.defers_annotations()
    }

    fn visit_function_def(&mut self, stmt: &'a Stmt, function_def: &'a ast::StmtFunctionDef) {
        let ast::StmtFunctionDef { name, parameters, decorator_list, type_params, .. } = function_def;
        for decorator in decorator_list {
            self.visit_decorator(decorator);
        }
        for default in parameters.iter().filter_map(AnyParameterRef::default) {
            self.visit_expr(default);
        }
        // A method sits in the implicit `__class__` closure, see `ScopeKind::DunderClassCell`.
        let in_class = self.semantic.current_scope().kind.is_class();
        if in_class {
            self.push_dunder_class_cell();
        }
        self.semantic.push_scope(ScopeKind::Type);
        if let Some(type_params) = type_params {
            self.visit_type_params(type_params);
        }
        let runtime = self.runtime.function(function_def, &self.semantic);
        for annotation in signature_annotations(function_def) {
            self.visit_annotation_as(annotation, runtime);
        }
        self.semantic.push_scope(ScopeKind::Function(function_def));
        self.functions.push((function_def, self.semantic.snapshot()));
        let scope_id = self.semantic.scope_id;
        self.semantic.pop_scope();
        self.semantic.pop_scope();
        if in_class {
            self.semantic.pop_scope();
        }
        self.add_binding(name, stmt.identifier(), BindingKind::FunctionDefinition(scope_id));
    }

    fn push_dunder_class_cell(&mut self) {
        self.semantic.push_scope(ScopeKind::DunderClassCell);
        self.declare("__class__", TextRange::default(), BindingKind::DunderClassCell, BindingFlags::empty());
    }

    fn declare(&mut self, name: &'a str, range: TextRange, kind: BindingKind<'a>, flags: BindingFlags) {
        let binding_id = self.semantic.push_binding(name, range, kind, flags);
        self.semantic.current_scope_mut().add(name, binding_id);
    }

    fn visit_generators(&mut self, kind: GeneratorKind, range: TextRange, generators: &'a [Comprehension]) {
        // The first iterable evaluates in the enclosing scope; everything after it in the comprehension's own scope.
        self.comprehensions.push((range, generators[0].iter.range()));
        self.visit_expr(&generators[0].iter);
        let is_async = generators.iter().any(|generator| generator.is_async);
        self.semantic.push_scope(ScopeKind::Generator { kind, is_async });
        for (index, generator) in generators.iter().enumerate() {
            if index > 0 {
                self.visit_expr(&generator.iter);
            }
            self.visit_expr(&generator.target);
            for condition in &generator.ifs {
                self.visit_expr(condition);
            }
        }
    }

    // Function bodies and lambdas run after the code around them binds its names, so they are visited last.
    fn visit_deferred(&mut self) {
        while !self.functions.is_empty() || !self.lambdas.is_empty() {
            for (function_def, snapshot) in std::mem::take(&mut self.functions) {
                self.semantic.restore(snapshot);
                self.visit_parameters(&function_def.parameters);
                self.visit_body(&function_def.body);
            }
            for (lambda, snapshot) in std::mem::take(&mut self.lambdas) {
                self.semantic.restore(snapshot);
                // A lambda in an annotation runs when called, not when the annotation is evaluated.
                self.semantic.flags -= SemanticModelFlags::ANNOTATION;
                if let Some(parameters) = &lambda.parameters {
                    self.visit_parameters(parameters);
                }
                let in_class = self.semantic.current_scopes().any(|scope| scope.kind.is_class());
                if in_class {
                    self.push_dunder_class_cell();
                }
                self.visit_expr(&lambda.body);
                if in_class {
                    self.semantic.pop_scope();
                }
            }
        }
    }

    fn add_binding(&mut self, name: &'a str, range: TextRange, kind: BindingKind<'a>) {
        // PEP 572: a named expression in a comprehension binds in the scope that contains the outermost comprehension.
        let scope_id = if kind.is_named_expr_assignment() {
            self.semantic
                .scopes
                .ancestor_ids(self.semantic.scope_id)
                .find(|scope_id| !self.semantic.scopes[*scope_id].kind.is_generator())
                .unwrap_or(ScopeId::global())
        } else {
            self.semantic.scope_id
        };
        let binding_id = self.semantic.push_binding(name, range, kind, BindingFlags::empty());
        // A later annotation of a bound name leaves its value alone, so it annotates the binding instead of shadowing it.
        if let Some(shadowed_id) = self.semantic.scopes[scope_id].get(name)
            && self.semantic.binding(binding_id).kind.is_annotation()
        {
            self.semantic.add_delayed_annotation(shadowed_id, binding_id);
            return;
        }
        self.semantic.scopes[scope_id].add(name, binding_id);
    }

    fn handle_node_store(&mut self, name: &'a ast::ExprName) {
        let parent = self.semantic.current_statement();
        let kind = if self.semantic.in_named_expression_assignment() {
            BindingKind::NamedExprAssignment
        } else if matches!(parent, Stmt::AnnAssign(ast::StmtAnnAssign { value: None, .. }))
            && !self.semantic.in_annotation()
        {
            BindingKind::Annotation
        } else if parent.is_for_stmt() {
            BindingKind::LoopVar
        } else if parent.is_with_stmt() {
            BindingKind::WithItemVar
        } else {
            BindingKind::Assignment
        };
        if matches!(kind, BindingKind::Annotation)
            && self.semantic.flags.contains(SemanticModelFlags::TYPE_CHECKING_BLOCK)
        {
            self.type_checking_stubs.insert(name.range);
        }
        self.add_binding(&name.id, name.range, kind);
    }

    // Unlike Ruff, a `del` in a conditional branch also unbinds the name: the rules count every `del` as a rebinding.
    fn handle_node_delete(&mut self, name: &'a ast::ExprName) {
        self.semantic.resolve_del(&name.id, name.range);
        let binding_id = self.semantic.push_binding(&name.id, name.range, BindingKind::Deletion, BindingFlags::empty());
        self.semantic.current_scope_mut().add(&name.id, binding_id);
    }
}
