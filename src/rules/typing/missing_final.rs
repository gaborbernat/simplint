use std::cell::OnceCell;

use ruff_diagnostics::{Edit, Fix, IsolationLevel};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::parenthesize::parenthesized_range;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::ensure_import;
use crate::rules::{has_version_gate, is_class_like, is_sphinx_conf, isort_key, single_assignment};
use crate::semantic::Semantic;
use crate::syntax::{alias_binding, last_segment};

use super::is_type_parameter_constructor;

pub fn missing_final(checker: &Checker) {
    // `typing.Final` arrived in Python 3.8.
    // Sphinx reads the module variables of `conf.py` by name, so a `Final` there would change nothing.
    if checker.settings.from_ruff.target_version < PythonVersion::PY38
        || has_version_gate(checker.body())
        || is_sphinx_conf(checker)
    {
        return;
    }
    let spelling = OnceCell::new();
    let globals_writes = OnceCell::new();
    for stmt in checker.body() {
        let Some((target, annotation, value)) = single_assignment(stmt) else {
            continue;
        };
        let name = target.id.as_str();
        // The annotation already declares a constant, so every name qualifies, dunders and class-like names included.
        if let Some(annotation) = annotation.filter(|annotation| is_bare_final(annotation)) {
            let fix = bare_final_fix(checker, annotation, value);
            checker.report_fix(
                Rule::MissingFinal,
                target.range,
                format!("`{name}` is annotated bare `Final`; give it a type, as in `Final[int]`"),
                fix,
                format!("Give `{name}` its type inside `Final`"),
            );
            continue;
        }
        if is_dunder(name)
            || is_class_like(name)
            || name == "TYPE_CHECKING"
            || is_type_alias(value)
            || checker.project().is_attribute_assigned(name)
            || checker.project().is_module_written(name)
            // A computed write may set any name of the module, as `setattr(module, name, value)` does.
            || checker.project().is_computed_written()
            || globals_writes.get_or_init(|| GlobalsWrites::of(checker)).writes(name)
            || checker.semantic().is_rebound(name)
            || annotation.is_some_and(is_qualifier)
            || is_type_construct(checker.semantic(), value)
        {
            continue;
        }
        // `(LIMIT: Final[int]) = 3` is not valid Python, so a parenthesized name gets no annotation of its own.
        let parenthesized = annotation.is_none()
            && parenthesized_range(target.into(), stmt.into(), checker.comment_ranges(), checker.source).is_some();
        let fix = (!parenthesized)
            .then(|| {
                spelling.get_or_init(|| final_qualifier(checker)).as_ref().and_then(|(qualifier, import)| {
                    final_fix(checker, qualifier, import.as_ref(), target.range, annotation, value)
                })
            })
            .flatten();
        checker.report_fix(
            Rule::MissingFinal,
            target.range,
            format!("`{name}` is never reassigned; annotate it `Final`"),
            fix,
            format!("Annotate `{name}` with `Final`"),
        );
    }
}

// The names a module writes through `globals()`, or through `vars()` and `locals()` with no argument at module level,
// and whether a write may set any name: a computed key, or the `update`, `setdefault` or `pop` of the dict, names the
// module cannot list.
struct GlobalsWrites<'a, 'c> {
    checker: &'c Checker<'a>,
    names: FxHashSet<&'a str>,
    any: bool,
    // The bodies of the functions, classes and lambdas the walk is in, whose `vars()` and `locals()` are not the module's.
    scopes: u32,
}

impl<'a, 'c> GlobalsWrites<'a, 'c> {
    fn of(checker: &'c Checker<'a>) -> Self {
        let mut writes = Self { checker, names: FxHashSet::default(), any: false, scopes: 0 };
        writes.visit_body(checker.body());
        writes
    }

    fn writes(&self, name: &str) -> bool {
        self.any || self.names.contains(name)
    }

    fn is_globals_call(&self, expr: &Expr) -> bool {
        let semantic = self.checker.semantic();
        expr.as_call_expr().is_some_and(|call| {
            semantic.is_builtin(&call.func, "globals")
                || self.scopes == 0
                    && call.arguments.is_empty()
                    && (semantic.is_builtin(&call.func, "vars") || semantic.is_builtin(&call.func, "locals"))
        })
    }
}

impl<'a> GlobalsWrites<'a, '_> {
    // The body of a function, class or lambda is its own scope, while its header runs in the enclosing one.
    fn scoped_body(&mut self, body: &'a [Stmt]) {
        self.scopes += 1;
        self.visit_body(body);
        self.scopes -= 1;
    }
}

impl<'a> Visitor<'a> for GlobalsWrites<'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef {
                parameters, body, decorator_list, returns, type_params, ..
            }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = type_params {
                    self.visit_type_params(type_params);
                }
                self.visit_parameters(parameters);
                if let Some(returns) = returns {
                    self.visit_annotation(returns);
                }
                self.scoped_body(body);
            }
            Stmt::ClassDef(ast::StmtClassDef { arguments, body, decorator_list, type_params, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(type_params) = type_params {
                    self.visit_type_params(type_params);
                }
                if let Some(arguments) = arguments {
                    self.visit_arguments(arguments);
                }
                self.scoped_body(body);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Lambda(ast::ExprLambda { parameters, body, .. }) = expr {
            if let Some(parameters) = parameters {
                self.visit_parameters(parameters);
            }
            self.scopes += 1;
            self.visit_expr(body);
            self.scopes -= 1;
            return;
        }
        match expr {
            Expr::Subscript(ast::ExprSubscript {
                value, slice, ctx: ExprContext::Store | ExprContext::Del, ..
            }) if self.is_globals_call(value) => match slice.as_string_literal_expr() {
                Some(key) => _ = self.names.insert(key.value.to_str()),
                None => self.any = true,
            },
            Expr::Call(ast::ExprCall { func, .. }) => {
                if let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = func.as_ref()
                    && matches!(attr.as_str(), "update" | "setdefault" | "pop")
                    && self.is_globals_call(value)
                {
                    self.any = true;
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

// `Final` with no brackets, spelled `Final`, `typing.Final` or an alias such as `t.Final`.
fn is_bare_final(annotation: &Expr) -> bool {
    last_segment(annotation) == Some("Final")
}

fn is_qualifier(annotation: &Expr) -> bool {
    if let Expr::StringLiteral(literal) = annotation {
        let base = literal.value.to_str().trim().split('[').next().unwrap_or_default();
        return matches!(base.rsplit('.').next(), Some("Final" | "ClassVar" | "TypeAlias"));
    }
    let (base, slice) = match annotation {
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => (value.as_ref(), Some(slice.as_ref())),
        other => (other, None),
    };
    match last_segment(base) {
        Some("Final" | "ClassVar" | "TypeAlias") => true,
        // `Annotated[Final[int], ...]` carries its qualifier in the first argument.
        Some("Annotated") => slice
            .map(|slice| slice.as_tuple_expr().and_then(|tuple| tuple.elts.first()).unwrap_or(slice))
            .is_some_and(is_qualifier),
        _ => false,
    }
}

// Type checkers reject `Final` on these, since they declare types rather than hold values.
fn is_type_construct(semantic: &Semantic, value: &Expr) -> bool {
    const CONSTRUCTS: &[&str] = &["NewType", "TypeAliasType", "NamedTuple", "TypedDict"];
    value.as_call_expr().is_some_and(|call| {
        is_type_parameter_constructor(semantic, &call.func)
            || CONSTRUCTS.iter().any(|member| semantic.match_typing_expr(&call.func, member))
    })
}

// An unannotated assignment of a type, such as `JSON = dict[str, Any]` or `_PathLike = str | bytes`, is an implicit type
// alias, which `Final` would turn into a variable that type expressions cannot use.
fn is_type_alias(value: &Expr) -> bool {
    const GENERICS: &[&str] = &[
        "dict",
        "list",
        "set",
        "frozenset",
        "tuple",
        "type",
        "Literal",
        "Union",
        "Optional",
        "Callable",
        "Annotated",
        "Mapping",
        "Sequence",
        "Iterable",
        "Iterator",
        "MutableMapping",
        "MutableSequence",
        "Generator",
    ];
    const TYPES: &[&str] = &["str", "bytes", "int", "float", "bool", "complex", "object", "Any"];
    match value {
        Expr::Subscript(ast::ExprSubscript { value, .. }) => {
            last_segment(value).is_some_and(|last| GENERICS.contains(&last))
        }
        Expr::BinOp(ast::ExprBinOp { left, op: ast::Operator::BitOr, right, .. }) => {
            [left, right].into_iter().all(|side| {
                matches!(side.as_ref(), Expr::NoneLiteral(_))
                    || side.as_name_expr().is_some_and(|name| TYPES.contains(&name.id.as_str()))
                    || is_type_alias(side)
            })
        }
        _ => false,
    }
}

fn final_fix(
    checker: &Checker,
    qualifier: &str,
    import: Option<&Edit>,
    target: TextRange,
    annotation: Option<&Expr>,
    value: &Expr,
) -> Option<Fix> {
    let edit = if let Some(annotation) = annotation {
        Edit::range_replacement(format!("{qualifier}[{}]", checker.text(annotation.range())), annotation.range())
    } else {
        let literal_type = literal_type(value)?;
        Edit::insertion(format!(": {qualifier}[{literal_type}]"), target.end())
    };
    Some(match import {
        // One import-adding fix per pass, so the next pass sees the import instead of adding it twice.
        Some(import) => Fix::unsafe_edits(edit, [import.clone()]).isolate(IsolationLevel::Group(1)),
        None => Fix::unsafe_edit(edit),
    })
}

// The type of a literal value, which is what `Final` infers from a bare assignment.
fn literal_type(value: &Expr) -> Option<&'static str> {
    Some(match value {
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(_), .. }) => "int",
        Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Float(_), .. }) => "float",
        Expr::StringLiteral(_) | Expr::FString(_) => "str",
        Expr::BytesLiteral(_) => "bytes",
        Expr::BooleanLiteral(_) => "bool",
        _ => return None,
    })
}

// A type checker infers `Final` on a bare literal as `Literal[1]`, which `Final[int]` widens, so the fix is unsafe.
fn bare_final_fix(checker: &Checker, annotation: &Expr, value: &Expr) -> Option<Fix> {
    let literal_type = literal_type(value)?;
    let edit =
        Edit::range_replacement(format!("{}[{literal_type}]", checker.text(annotation.range())), annotation.range());
    Some(Fix::unsafe_edit(edit))
}

// The name a fix annotates with, and the import it adds; `None` when the import cannot be added without shadowing.
fn final_qualifier(checker: &Checker) -> Option<(String, Option<Edit>)> {
    let body = checker.body();
    // A `Final` bound only under `if TYPE_CHECKING:` is missing at runtime, where eager annotations evaluate it.
    let lazy = checker.semantic().annotations_are_lazy();
    let bound_at_top = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias_binding(alias) == "Final")
        }
        _ => false,
    });
    if checker.semantic().is_bound("Final") && (lazy || bound_at_top) {
        return Some(("Final".to_owned(), None));
    }
    let typing_module = body
        .iter()
        .filter_map(Stmt::as_import_stmt)
        .flat_map(|import| &import.names)
        .find(|alias| alias.name.as_str() == "typing");
    if let Some(alias) = typing_module {
        return Some((format!("{}.Final", alias.asname.as_ref().unwrap_or(&alias.name)), None));
    }
    let existing = body.iter().find_map(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. })
            if module.as_str() == "typing" && names.first().is_some_and(|alias| alias.name.as_str() != "*") =>
        {
            Some(names)
        }
        _ => None,
    });
    let edit = match existing {
        // Ruff's isort orders names by type, constants such as `TYPE_CHECKING` before classes such as `Final`, then
        // alphabetically, so `Final` goes before the first name that sorts after it.
        Some(names) => match names.iter().find(|alias| isort_key(alias.name.as_str()) > isort_key("Final")) {
            Some(alias) => Some(Edit::insertion("Final, ".to_owned(), alias.start())),
            None => Some(Edit::insertion(", Final".to_owned(), names.last().expect("an import names something").end())),
        },
        None => ensure_import(checker, "typing", Some("Final"))?.1,
    };
    Some(("Final".to_owned(), edit))
}
