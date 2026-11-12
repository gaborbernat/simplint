use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Operator, Stmt};
use ruff_text_size::Ranged;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::project::constructors::{CallShape, CallSites};
use crate::rules::call_sites::{OwnCalls, candidate_functions, judges_call_sites};
use crate::semantic::{Referent, binding_referent};
use crate::syntax::{is_name, last_segment};

pub fn test_only_parameter(checker: &Checker) {
    if !judges_call_sites(checker) {
        return;
    }
    // Built once per module: building it walks the module, and every function's lookup reuses it.
    let own = OwnCalls::of(checker);
    for function in candidate_functions(checker) {
        let sites = own.function(checker, function.name.as_str());
        // A call that unpacks `**kwargs` may pass any parameter, and a call the index cannot see hides what it passes.
        // A program's entry gets its arguments from the command line, so leaving one out there says nothing.
        if sites.opaque
            || sites.entry
            || sites.production.is_empty()
            || sites.production.iter().any(|shape| shape.unpacks_mapping)
        {
            continue;
        }
        report_parameters(checker, function, &sites);
    }
}

// Only a default can be left out, so a required parameter is never reported.
fn report_parameters(checker: &Checker, function: &ast::StmtFunctionDef, sites: &CallSites) {
    let parameters = &function.parameters;
    for (index, param) in parameters.posonlyargs.iter().chain(&parameters.args).enumerate() {
        if let Some(default) = param.default.as_deref()
            && !is_injected_dependency(checker, function, param, default)
        {
            let name = param.parameter.name.as_str();
            let passed = |shape: &CallShape| {
                index < shape.positional as usize
                    || (index >= parameters.posonlyargs.len() && shape.keywords.iter().any(|keyword| keyword == name))
            };
            report_if_test_only(checker, function, param, sites, passed);
        }
    }
    for param in &parameters.kwonlyargs {
        if let Some(default) = param.default.as_deref()
            && !is_injected_dependency(checker, function, param, default)
        {
            let passed =
                |shape: &CallShape| shape.keywords.iter().any(|keyword| keyword == param.parameter.name.as_str());
            report_if_test_only(checker, function, param, sites, passed);
        }
    }
}

// A defaulted parameter that passes in a collaborator is a seam, not test plumbing, so the rule leaves it alone. A
// collaborator is a function, a class, a module, or an instance built by calling a class. A `None` default names none,
// so it is exempt only when the body calls the parameter, or when the annotation names a class and the body uses it.
// Flags, literals, data and enums stay reported.
fn is_injected_dependency(
    checker: &Checker,
    function: &ast::StmtFunctionDef,
    param: &ast::ParameterWithDefault,
    default: &Expr,
) -> bool {
    let name = param.parameter.name.as_str();
    let annotation = param.parameter.annotation.as_deref();
    if annotation.is_some_and(|annotation| is_enum_annotation(checker, annotation)) || is_enum_default(checker, default)
    {
        return false;
    }
    if default.is_none_literal_expr() {
        let by_annotation = annotation.is_some_and(|annotation| is_class_annotation(checker, annotation));
        return is_used(function, name, ParameterUse::Called)
            || (by_annotation && is_used(function, name, ParameterUse::CalledOrRead));
    }
    // A name may be bound to another name, so a chain of assignments is followed this many steps before it is unresolved.
    is_collaborator(checker, default, 8) && is_used(function, name, ParameterUse::CalledOrRead)
}

fn is_enum_annotation(checker: &Checker, annotation: &Expr) -> bool {
    match annotation {
        Expr::BinOp(ast::ExprBinOp { left, right, op: Operator::BitOr, .. }) => {
            [left.as_ref(), right.as_ref()].into_iter().any(|side| is_enum_annotation(checker, side))
        }
        Expr::Subscript(ast::ExprSubscript { value, slice, .. })
            if checker.semantic().match_typing_expr(value, "Optional") =>
        {
            is_enum_annotation(checker, slice)
        }
        _ => is_enum_expr(checker, annotation),
    }
}

// An enum the project defines or imports, or one a module of the run defines, named through its module.
fn is_enum_expr(checker: &Checker, expr: &Expr) -> bool {
    let project = checker.project();
    match expr {
        Expr::Name(name) => project.is_enum_class(name.id.as_str()),
        Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => match value.as_ref() {
            Expr::Name(module) => project.is_run_module_enum(module.id.as_str(), attr.as_str()),
            _ => false,
        },
        _ => false,
    }
}

// A member of an enum, or a value an enum builds, is a flag.
fn is_enum_default(checker: &Checker, default: &Expr) -> bool {
    match default {
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_enum_expr(checker, value),
        Expr::Call(ast::ExprCall { func, .. }) => is_enum_expr(checker, func),
        _ => false,
    }
}

// The class an annotation names, unwrapped from `Optional[...]` and `... | None`. A builtin or collection type is data,
// and `Literal[...]` is a value, so neither is a class.
fn is_class_annotation(checker: &Checker, annotation: &Expr) -> bool {
    match annotation {
        Expr::BinOp(ast::ExprBinOp { left, right, op: Operator::BitOr, .. }) => [left.as_ref(), right.as_ref()]
            .into_iter()
            .filter(|side| !side.is_none_literal_expr())
            .all(|side| is_class_annotation(checker, side)),
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            if checker.semantic().match_typing_expr(value, "Optional") {
                is_class_annotation(checker, slice)
            } else {
                !checker.semantic().match_typing_expr(value, "Literal") && !is_data_type(value)
            }
        }
        Expr::Name(_) | Expr::Attribute(_) => !is_data_type(annotation) && !is_enum_expr(checker, annotation),
        _ => false,
    }
}

// The builtin and collection types a parameter holds data in, named by their last segment so `collections.abc.Mapping`
// matches too. Any other name, such as `ModuleType` or a protocol, is taken for a class.
fn is_data_type(expr: &Expr) -> bool {
    const DATA_TYPES: &[&str] = &[
        "Any",
        "AbstractSet",
        "AsyncGenerator",
        "AsyncIterable",
        "AsyncIterator",
        "bool",
        "bytearray",
        "bytes",
        "complex",
        "Collection",
        "Container",
        "Counter",
        "DefaultDict",
        "Deque",
        "dict",
        "Dict",
        "float",
        "frozenset",
        "FrozenSet",
        "Generator",
        "Hashable",
        "int",
        "ItemsView",
        "Iterable",
        "Iterator",
        "KeysView",
        "list",
        "List",
        "Mapping",
        "MappingView",
        "MutableMapping",
        "MutableSequence",
        "MutableSet",
        "object",
        "OrderedDict",
        "Reversible",
        "Sequence",
        "set",
        "Set",
        "Sized",
        "str",
        "tuple",
        "Tuple",
        "ValuesView",
        "ChainMap",
    ];
    last_segment(expr).is_some_and(|segment| DATA_TYPES.contains(&segment))
}

fn is_used(function: &ast::StmtFunctionDef, name: &str, uses: ParameterUse) -> bool {
    let mut usage = ObjectUse { name, uses, found: false };
    usage.visit_body(&function.body);
    usage.found
}

// Whether a default names a collaborator: a function, a class, a module, or an instance built by calling a class.
fn is_collaborator(checker: &Checker, expr: &Expr, depth: u8) -> bool {
    match expr {
        Expr::Name(name) => depth > 0 && name_is_collaborator(checker, name, depth),
        Expr::Attribute(attribute) => attribute_is_collaborator(checker, attribute),
        Expr::Call(ast::ExprCall { func, .. }) => constructs_class(checker, func),
        Expr::Lambda(_) => true,
        _ => false,
    }
}

// A name is a collaborator when every binding it resolves to is one. An assigned name is one when its values are.
fn name_is_collaborator(checker: &Checker, name: &ast::ExprName, depth: u8) -> bool {
    let semantic = checker.semantic();
    let bindings = semantic.resolved_bindings(name);
    !bindings.is_empty()
        && bindings.iter().all(|binding| match binding_referent(binding) {
            Referent::Function | Referent::Class | Referent::Module | Referent::Builtin => true,
            Referent::Member => checker.project().run_import_class(name.id.as_str()).is_some(),
            Referent::Value => semantic.binding_values(binding).is_some_and(|values| {
                !values.is_empty() && values.into_iter().all(|value| is_collaborator(checker, value, depth - 1))
            }),
            Referent::Other => false,
        })
}

// A member of a module the run defines, or a method of a class the module defines.
fn attribute_is_collaborator(checker: &Checker, attribute: &ast::ExprAttribute) -> bool {
    let Expr::Name(base) = attribute.value.as_ref() else {
        return false;
    };
    let member = attribute.attr.as_str();
    if is_module(checker, base) {
        return checker.project().run_module_member(base.id.as_str(), member).is_some();
    }
    let semantic = checker.semantic();
    let bindings = semantic.resolved_bindings(base);
    !bindings.is_empty()
        && bindings
            .iter()
            .all(|binding| semantic.class_of_binding(binding).is_some_and(|class| defines_method(class, member)))
}

fn is_module(checker: &Checker, name: &ast::ExprName) -> bool {
    let bindings = checker.semantic().resolved_bindings(name);
    !bindings.is_empty() && bindings.iter().all(|binding| binding_referent(binding) == Referent::Module)
}

fn defines_method(class: &ast::StmtClassDef, name: &str) -> bool {
    class.body.iter().filter_map(Stmt::as_function_def_stmt).any(|method| method.name.as_str() == name)
}

// A call builds an instance of a class the project defines or imports, or of a builtin class. A lowercase builtin, such
// as `dict()`, builds data.
fn constructs_class(checker: &Checker, func: &Expr) -> bool {
    match func {
        Expr::Name(name) => {
            let semantic = checker.semantic();
            let bindings = semantic.resolved_bindings(name);
            !bindings.is_empty()
                && bindings.iter().all(|binding| match binding_referent(binding) {
                    Referent::Class => true,
                    Referent::Member => checker.project().run_import_class(name.id.as_str()) == Some(true),
                    Referent::Builtin => name.id.starts_with(char::is_uppercase),
                    _ => false,
                })
        }
        Expr::Attribute(attribute) => match attribute.value.as_ref() {
            Expr::Name(base) => {
                is_module(checker, base)
                    && checker.project().run_module_member(base.id.as_str(), attribute.attr.as_str()) == Some(true)
            }
            _ => false,
        },
        _ => false,
    }
}

// The uses a parameter's body makes of it: a call of it, and, for a reference default, an attribute read on it too.
#[derive(Clone, Copy, PartialEq)]
enum ParameterUse {
    Called,
    CalledOrRead,
}

struct ObjectUse<'n> {
    name: &'n str,
    uses: ParameterUse,
    found: bool,
}

impl<'a> Visitor<'a> for ObjectUse<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        let object_use = match expr {
            Expr::Call(ast::ExprCall { func, .. }) => is_name(func, self.name),
            Expr::Attribute(ast::ExprAttribute { value, .. }) => {
                self.uses == ParameterUse::CalledOrRead && is_name(value, self.name)
            }
            _ => false,
        };
        if object_use {
            self.found = true;
        } else {
            visitor::walk_expr(self, expr);
        }
    }
}

fn report_if_test_only(
    checker: &Checker,
    function: &ast::StmtFunctionDef,
    param: &ast::ParameterWithDefault,
    sites: &CallSites,
    passed: impl Fn(&CallShape) -> bool,
) {
    if sites.production.iter().any(&passed) || !sites.shapes.iter().any(&passed) {
        return;
    }
    let name = param.parameter.name.as_str();
    checker.report(
        Rule::TestOnlyParameter,
        param.parameter.name.range(),
        format!("Only tests pass `{name}` to `{}`; production calls leave it at its default", function.name.as_str()),
    );
}
