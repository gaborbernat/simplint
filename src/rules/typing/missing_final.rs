use ruff_diagnostics::{Edit, Fix, IsolationLevel};
use ruff_python_ast::helpers::{is_docstring_stmt, is_dunder};
use ruff_python_ast::{self as ast, Expr, PythonVersion, Stmt};
use ruff_python_stdlib::sys::is_known_standard_library;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::{is_class_like, last_segment, single_assignment};
use crate::semantic::{Semantic, future_import};

pub fn missing_final(checker: &Checker) {
    // `typing.Final` arrived in Python 3.8.
    if checker.settings.from_ruff.target_version < PythonVersion::PY38 {
        return;
    }
    for stmt in checker.body() {
        let Some((target, annotation, value)) = single_assignment(stmt) else {
            continue;
        };
        let name = target.id.as_str();
        if is_dunder(name)
            || is_class_like(name)
            || name == "TYPE_CHECKING"
            || is_type_alias(value)
            || checker.project().is_attribute_assigned(name)
            || checker.semantic().is_rebound(name)
            || annotation.is_some_and(is_qualifier)
            || is_type_construct(checker.semantic(), value)
        {
            continue;
        }
        let fix = final_fix(checker, target.range, annotation, value);
        checker.report_fix(
            Rule::MissingFinal,
            target.range,
            format!("`{name}` is never reassigned; annotate it `Final`"),
            fix,
            format!("Annotate `{name}` with `Final`"),
        );
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
    const CONSTRUCTS: &[&str] =
        &["TypeVar", "ParamSpec", "TypeVarTuple", "NewType", "TypeAliasType", "NamedTuple", "TypedDict"];
    value
        .as_call_expr()
        .is_some_and(|call| CONSTRUCTS.iter().any(|member| semantic.match_typing_expr(&call.func, member)))
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

fn final_fix(checker: &Checker, target: TextRange, annotation: Option<&Expr>, value: &Expr) -> Option<Fix> {
    let (qualifier, import) = final_qualifier(checker);
    let edit = if let Some(annotation) = annotation {
        Edit::range_replacement(format!("{qualifier}[{}]", checker.text(annotation.range())), annotation.range())
    } else {
        let literal_type = match value {
            Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Int(_), .. }) => "int",
            Expr::NumberLiteral(ast::ExprNumberLiteral { value: ast::Number::Float(_), .. }) => "float",
            Expr::StringLiteral(_) | Expr::FString(_) => "str",
            Expr::BytesLiteral(_) => "bytes",
            Expr::BooleanLiteral(_) => "bool",
            _ => return None,
        };
        Edit::insertion(format!(": {qualifier}[{literal_type}]"), target.end())
    };
    Some(match import {
        // One import-adding fix per pass, so the next pass sees the import instead of adding it twice.
        Some(import) => Fix::unsafe_edits(edit, [import]).isolate(IsolationLevel::Group(1)),
        None => Fix::unsafe_edit(edit),
    })
}

fn final_qualifier(checker: &Checker) -> (String, Option<Edit>) {
    let body = checker.body();
    // A `Final` bound only under `if TYPE_CHECKING:` is missing at runtime, where eager annotations evaluate it.
    let lazy = checker.semantic().annotations_are_lazy();
    let bound_at_top = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str() == "Final")
        }
        _ => false,
    });
    if checker.semantic().is_bound("Final") && (lazy || bound_at_top) {
        return ("Final".to_owned(), None);
    }
    let typing_module = body
        .iter()
        .filter_map(Stmt::as_import_stmt)
        .flat_map(|import| &import.names)
        .find(|alias| alias.name.as_str() == "typing");
    if let Some(alias) = typing_module {
        return (format!("{}.Final", alias.asname.as_ref().unwrap_or(&alias.name)), None);
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
            Some(alias) => Edit::insertion("Final, ".to_owned(), alias.start()),
            None => Edit::insertion(", Final".to_owned(), names.last().expect("an import names something").end()),
        },
        None => typing_import(checker),
    };
    ("Final".to_owned(), Some(edit))
}

fn isort_key(name: &str) -> (u8, String) {
    let rank = if name.len() > 1 && !name.chars().any(char::is_lowercase) {
        0
    } else if name.starts_with(char::is_uppercase) {
        1
    } else {
        2
    };
    (rank, name.to_ascii_lowercase())
}

// The import goes where Ruff's isort keeps it: in the standard library section of the leading import block, after the
// plain `import x` lines and in module order among the `from x import y` lines, with a blank line around a new section.
fn typing_import(checker: &Checker) -> Edit {
    const IMPORT: &str = "from typing import Final\n";
    let body = checker.body();
    let source = checker.source;
    let preamble = body.iter().take_while(|stmt| future_import(stmt).is_some() || is_docstring_stmt(stmt)).count();
    let block: Vec<&Stmt> =
        body[preamble..].iter().take_while(|stmt| matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_))).collect();
    let minor = checker.settings.from_ruff.target_version.minor;
    let stdlib: Vec<(&Stmt, Option<&str>)> = block
        .iter()
        .filter_map(|stmt| match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) => names
                .iter()
                .all(|alias| is_known_standard_library(minor, alias.name.split('.').next().unwrap_or_default()))
                .then_some((*stmt, None)),
            Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
                is_known_standard_library(minor, module.split('.').next().unwrap_or_default())
                    .then_some((*stmt, Some(module.as_str())))
            }
            _ => None,
        })
        .collect();
    if let Some((next, _)) =
        stdlib.iter().find(|(_, module)| module.is_some_and(|module| module.to_ascii_lowercase().as_str() > "typing"))
    {
        return Edit::insertion(IMPORT.to_owned(), source.line_start(next.start()));
    }
    if let Some((last, _)) = stdlib.last() {
        return Edit::insertion(IMPORT.to_owned(), source.full_line_end(last.end()));
    }
    // The variable that needs `Final` follows the preamble, so the body has a statement there.
    Edit::insertion(format!("{IMPORT}\n"), source.line_start(body[preamble].start()))
}
