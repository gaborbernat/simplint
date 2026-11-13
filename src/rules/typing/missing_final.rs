use std::cell::OnceCell;

use ruff_diagnostics::{Applicability, Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::parenthesize::parenthesized_range;
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::{Import, ensure_import, with_import};
use crate::rules::{is_class_like, is_sphinx_conf, isort_key};
use crate::semantic::Semantic;
use crate::syntax::{alias_binding, from_imports, has_version_gate, last_segment, single_assignment};

use super::globals_writes::GlobalsWrites;
use super::is_type_parameter_constructor;

pub fn missing_final(checker: &Checker) {
    // `typing.Final` arrived in Python 3.8.
    // Sphinx reads the module variables of `conf.py` by name, so a `Final` there would change nothing.
    if checker.target_version() < PythonVersion::PY38 || has_version_gate(checker.body()) || is_sphinx_conf(checker) {
        return;
    }
    let final_spelling = OnceCell::new();
    let globals_writes = OnceCell::new();
    for stmt in checker.body() {
        let Some((target, annotation, value)) = single_assignment(stmt) else {
            continue;
        };
        let name = target.id.as_str();
        // The annotation already declares a constant, so every name qualifies, dunders and class-like names included.
        if let Some(annotation) = annotation.filter(|annotation| is_bare_final(annotation)) {
            checker.report_fix(
                Rule::MissingFinal,
                target.range,
                format!("`{name}` is annotated bare `Final`; give it a type, as in `Final[int]`"),
                bare_final_fix(checker, annotation, value),
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
        // Every target is a statement of the module body, where no scope encloses the import, so the file's start
        // stands for all of them and one answer serves every target.
        let fix = (!parenthesized)
            .then(|| {
                final_spelling
                    .get_or_init(|| final_qualifier(checker, TextRange::default()))
                    .as_ref()
                    .and_then(|spelling| final_fix(checker, spelling, target.range, annotation, value))
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

// `Final` with no brackets, spelled `Final`, `typing.Final` or an alias such as `t.Final`.
fn is_bare_final(annotation: &Expr) -> bool {
    last_segment(annotation) == Some("Final")
}

// A type checker infers `Final` on a bare literal as `Literal[1]`, which `Final[int]` widens, so the fix is unsafe.
fn bare_final_fix(checker: &Checker, annotation: &Expr, value: &Expr) -> Option<Fix> {
    let literal_type = literal_type(value)?;
    let edit =
        Edit::range_replacement(format!("{}[{literal_type}]", checker.text(annotation.range())), annotation.range());
    Some(Fix::unsafe_edit(edit))
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

// An unannotated assignment of a type, such as `JSON = dict[str, Any]` or `_PathLike = str | bytes`, is an implicit
// type alias, which `Final` would turn into a variable that type expressions cannot use.
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

pub(crate) struct Spelling {
    pub(crate) qualifier: String,
    pub(crate) import: Import,
}

// The name a fix annotates with, and the import it adds; `None` when the import cannot be added without shadowing.
pub(crate) fn final_qualifier(checker: &Checker, at: TextRange) -> Option<Spelling> {
    let body = checker.body();
    let bound_at_top = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias_binding(alias) == "Final")
        }
        _ => false,
    });
    // A `Final` bound only under `if TYPE_CHECKING:` is missing at runtime, where eager annotations evaluate it.
    if checker.semantic().is_bound("Final") && (checker.semantic().annotations_are_lazy() || bound_at_top) {
        return Some(Spelling { qualifier: "Final".to_owned(), import: Import::Present });
    }
    let typing_module = body
        .iter()
        .filter_map(Stmt::as_import_stmt)
        .flat_map(|import| &import.names)
        .find(|alias| alias.name.as_str() == "typing");
    if let Some(alias) = typing_module {
        return Some(Spelling {
            qualifier: format!("{}.Final", alias.asname.as_ref().unwrap_or(&alias.name)),
            import: Import::Present,
        });
    }
    let existing = from_imports(body, "typing")
        .map(|import| &import.names)
        .find(|names| names.first().is_some_and(|alias| alias.name.as_str() != "*"));
    let import = match existing {
        // Ruff's isort orders names by type, constants such as `TYPE_CHECKING` before classes such as `Final`, then
        // alphabetically, so `Final` goes before the first name that sorts after it.
        Some(names) => {
            Import::Add(match names.iter().find(|alias| isort_key(alias.name.as_str()) > isort_key("Final")) {
                Some(alias) => Edit::insertion("Final, ".to_owned(), alias.start()),
                None => Edit::insertion(", Final".to_owned(), names.last().expect("an import names something").end()),
            })
        }
        None => ensure_import(checker, "typing", Some("Final"), at)?,
    };
    Some(Spelling { qualifier: "Final".to_owned(), import })
}

fn final_fix(
    checker: &Checker,
    spelling: &Spelling,
    target: TextRange,
    annotation: Option<&Expr>,
    value: &Expr,
) -> Option<Fix> {
    let edit = if let Some(annotation) = annotation {
        Edit::range_replacement(
            format!("{}[{}]", spelling.qualifier, checker.text(annotation.range())),
            annotation.range(),
        )
    } else {
        let literal_type = literal_type(value)?;
        Edit::insertion(format!(": {}[{literal_type}]", spelling.qualifier), target.end())
    };
    Some(with_import(edit, spelling.import.clone(), Applicability::Unsafe))
}
