use ruff_diagnostics::{Edit, Fix, IsolationLevel};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{
    Exports, appears_as_name, docstring, dunder_all, function_defs, indentation, is_alone_on_lines, is_class_like,
    lines_with_trailing_blanks, needs_parentheses,
};
use crate::semantic::{BindingKind, Evaluation};

pub fn missing_final(checker: &mut Checker) {
    let mut found = Vec::new();
    for stmt in checker.body() {
        let (target, annotation, value) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], None, value),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value: Some(value), .. }) => {
                (target.as_ref(), Some(annotation.as_ref()), value)
            }
            _ => continue,
        };
        let Some(name) = target.as_name_expr().map(|name| name.id.as_str()) else {
            continue;
        };
        if is_dunder(name)
            || is_class_like(name)
            || checker.semantic().is_rebound(name)
            || annotation.is_some_and(is_qualifier)
            || is_type_construct(value)
        {
            continue;
        }
        found.push((target.range(), name, annotation, value.as_ref()));
    }
    for (range, name, annotation, value) in found {
        let fix = final_fix(checker, range, annotation, value);
        checker.report(Rule::MissingFinal, range, format!("`{name}` is never reassigned; annotate it `Final`"), fix);
    }
}

fn is_qualifier(annotation: &Expr) -> bool {
    if let Expr::StringLiteral(literal) = annotation {
        let base = literal.value.to_str().trim().split('[').next().unwrap_or_default();
        return matches!(base.rsplit('.').next(), Some("Final" | "ClassVar" | "TypeAlias"));
    }
    let base = match annotation {
        Expr::Subscript(ast::ExprSubscript { value, .. }) => value.as_ref(),
        other => other,
    };
    UnqualifiedName::from_expr(base)
        .is_some_and(|name| matches!(name.segments().last(), Some(&("Final" | "ClassVar" | "TypeAlias"))))
}

// Type checkers reject `Final` on these, since they declare types rather than hold values.
fn is_type_construct(value: &Expr) -> bool {
    value.as_call_expr().and_then(|call| UnqualifiedName::from_expr(&call.func)).is_some_and(|name| {
        matches!(
            name.segments().last(),
            Some(
                &("TypeVar" | "ParamSpec" | "TypeVarTuple" | "NewType" | "TypeAliasType" | "NamedTuple" | "TypedDict")
            )
        )
    })
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
        Some(import) => Fix::safe_edits(edit, [import]).isolate(IsolationLevel::Group(1)),
        None => Fix::safe_edit(edit),
    })
}

fn final_qualifier(checker: &Checker) -> (&'static str, Option<Edit>) {
    let body = checker.body();
    // A `Final` bound only under `if TYPE_CHECKING:` is missing at runtime, where eager annotations evaluate it.
    let lazy = checker.annotations_are_lazy();
    let bound_at_top = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias.asname.as_ref().unwrap_or(&alias.name).as_str() == "Final")
        }
        _ => false,
    });
    if checker.semantic().is_bound("Final") && (lazy || bound_at_top) {
        return ("Final", None);
    }
    let typing_module = body
        .iter()
        .filter_map(Stmt::as_import_stmt)
        .flat_map(|import| &import.names)
        .any(|alias| alias.name.as_str() == "typing" && alias.asname.is_none());
    if typing_module {
        return ("typing.Final", None);
    }
    let existing = body.iter().find_map(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), names, level: 0, .. })
            if module.as_str() == "typing" && names.first().is_some_and(|alias| alias.name.as_str() != "*") =>
        {
            names.first()
        }
        _ => None,
    });
    let edit = match existing {
        Some(alias) => Edit::insertion("Final, ".to_owned(), alias.start()),
        None => Edit::insertion("from typing import Final\n".to_owned(), import_insertion_point(checker)),
    };
    ("Final", Some(edit))
}

fn import_insertion_point(checker: &Checker) -> TextSize {
    let body = checker.body();
    let source = checker.source;
    let preamble =
        body.iter().take_while(|stmt| is_future_import(stmt) || docstring(std::slice::from_ref(stmt)).is_some());
    match preamble.last() {
        Some(stmt) => source.full_line_end(stmt.end()),
        None => body.first().map_or(TextSize::default(), |stmt| source.line_start(stmt.start())),
    }
}

fn is_future_import(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), .. }) if module.as_str() == "__future__")
}

pub fn single_user_constant(checker: &mut Checker) {
    let body = checker.body();
    let exported = match dunder_all(body) {
        Exports::Listed(names) => Some(names),
        Exports::Absent | Exports::Dynamic => None,
    };
    let mut found = Vec::new();
    for stmt in body {
        let (target, value) = match stmt {
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], value.as_ref()),
            Stmt::AnnAssign(ast::StmtAnnAssign { target, value: Some(value), .. }) => (target.as_ref(), value.as_ref()),
            _ => continue,
        };
        let Some(name) = target.as_name_expr().map(|name| name.id.as_str()) else {
            continue;
        };
        let private = match &exported {
            Some(names) => !names.contains(&name),
            None => name.starts_with('_'),
        };
        if !private
            || checker.is_imported_elsewhere(name)
            || is_dunder(name)
            || is_class_like(name)
            || has_comment_above(checker.source, stmt)
        {
            continue;
        }
        let semantic = checker.semantic();
        if semantic.bindings.get(name).is_none_or(|bindings| {
            bindings.len() != 1 || !matches!(bindings[0].kind, BindingKind::Assignment | BindingKind::Annotated)
        }) {
            continue;
        }
        let references: Vec<_> = semantic.references_to(name).copied().collect();
        let Some(function) = references.first().and_then(|reference| reference.function) else {
            continue;
        };
        if references
            .iter()
            .all(|reference| reference.evaluation == Evaluation::Deferred && reference.function == Some(function))
        {
            found.push(Constant {
                stmt,
                name,
                value,
                function,
                references: references.into_iter().map(|reference| reference.range).collect(),
            });
        }
    }
    for constant in found {
        let fix = move_into_function(checker, &constant);
        checker.report(
            Rule::SingleUserConstant,
            constant.stmt.range(),
            format!("`{}` is used by one function only; define it there", constant.name),
            fix,
        );
    }
}

fn has_comment_above(source: &str, stmt: &Stmt) -> bool {
    let start = source.line_start(stmt.start());
    start.to_usize() > 0 && source.full_line_str(start - TextSize::from(1)).trim_start().starts_with('#')
}

struct Constant<'a> {
    stmt: &'a Stmt,
    name: &'a str,
    value: &'a Expr,
    function: TextRange,
    references: Vec<TextRange>,
}

// Deleting a statement leaves the blank lines on both sides of it next to each other; drop the shorter run so the code
// around it keeps the wider separation, and keep everything when the statement directly follows code.
fn deletion_keeping_separation(source: &str, range: TextRange) -> TextRange {
    let lines = source.full_lines_range(range);
    let before = &source[..lines.start().to_usize()];
    let leading_text = before.trim_end_matches([' ', '\t', '\n']);
    let leading = before[leading_text.len()..].matches('\n').count().saturating_sub(1);
    let with_trailing = lines_with_trailing_blanks(source, range);
    let trailing = source[lines.end().to_usize()..with_trailing.end().to_usize()].matches('\n').count();
    if leading_text.is_empty() || (leading > 0 && trailing <= leading) {
        with_trailing
    } else if leading > 0 {
        let line_end = leading_text.len() + before[leading_text.len()..].find('\n').map_or(0, |offset| offset + 1);
        TextRange::new(TextSize::try_from(line_end).unwrap_or_default(), lines.end())
    } else {
        lines
    }
}

fn move_into_function(checker: &Checker, constant: &Constant) -> Option<Fix> {
    let source = checker.source;
    if !is_alone_on_lines(source, constant.stmt.range()) {
        return None;
    }
    let deletion = Edit::range_deletion(deletion_keeping_separation(source, constant.stmt.range()));
    let value_text = checker.parenthesized_text(constant.value, constant.stmt.into());
    let function = function_defs(checker.body()).into_iter().find(|candidate| candidate.range == constant.function)?;
    if let [reference] = constant.references.as_slice()
        && !value_text.contains('\n')
        && !is_repeated(&function.body, *reference)
    {
        let inlined = if needs_parentheses(constant.value) && !value_text.starts_with('(') {
            format!("({value_text})")
        } else {
            value_text.to_owned()
        };
        return Some(Fix::unsafe_edits(deletion, [Edit::range_replacement(inlined, *reference)]));
    }
    let local = constant.name.trim_start_matches('_').to_ascii_lowercase();
    if local.is_empty() || appears_as_name(checker, &local) {
        return None;
    }
    let anchor = function.body[usize::from(docstring(&function.body).is_some())..]
        .iter()
        .find(|candidate| candidate.range().contains_range(constant.references[0]))?;
    let indent = indentation(source, anchor);
    let value_text = if value_text.contains("\"\"\"") || value_text.contains("'''") {
        value_text.to_owned()
    } else {
        value_text.replace('\n', &format!("\n{indent}"))
    };
    let declaration = Edit::insertion(format!("{local} = {value_text}\n{indent}"), anchor.start());
    let renames = constant.references.iter().map(|range| Edit::range_replacement(local.clone(), *range));
    Some(Fix::unsafe_edits(deletion, std::iter::once(declaration).chain(renames)))
}

fn is_repeated(body: &[Stmt], reference: TextRange) -> bool {
    let mut finder = RepeatedContext { reference, depth: 0, repeated: false };
    finder.visit_body(body);
    finder.repeated
}

struct RepeatedContext {
    reference: TextRange,
    depth: usize,
    repeated: bool,
}

impl<'a> Visitor<'a> for RepeatedContext {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let repeats = matches!(stmt, Stmt::For(_) | Stmt::While(_) | Stmt::FunctionDef(_) | Stmt::ClassDef(_));
        self.depth += usize::from(repeats);
        visitor::walk_stmt(self, stmt);
        self.depth -= usize::from(repeats);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if expr.range() == self.reference {
            self.repeated = self.depth > 0;
        }
        let repeats = matches!(
            expr,
            Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_)
        );
        self.depth += usize::from(repeats);
        visitor::walk_expr(self, expr);
        self.depth -= usize::from(repeats);
    }
}

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case(Rule::SingleUserConstant, "SMP050.py" ; "single_user_constant")]
    #[test_case(Rule::SingleUserConstant, "SMP050_all.py" ; "single_user_constant_with_all")]
    #[test_case(Rule::SingleUserConstant, "SMP050_dynamic.py" ; "single_user_constant_dynamic_all")]
    #[test_case(Rule::SingleUserConstant, "SMP050_spacing.py" ; "single_user_constant_spacing")]
    #[test_case(Rule::MissingFinal, "SMP101.py" ; "missing_final")]
    #[test_case(Rule::MissingFinal, "SMP101_typing.py" ; "missing_final_extends_typing_import")]
    #[test_case(Rule::MissingFinal, "SMP101_module.py" ; "missing_final_typing_module")]
    #[test_case(Rule::MissingFinal, "SMP101_new.py" ; "missing_final_new_import")]
    #[test_case(Rule::MissingFinal, "SMP101_bare.py" ; "missing_final_import_at_top")]
    fn rule(rule: Rule, fixture: &str) {
        check(rule, fixture, PythonVersion::PY314);
    }

    #[test_case("SMP101_checking.py", PythonVersion::PY310 ; "type_checking_final_eager")]
    #[test_case("SMP101_checking.py", PythonVersion::PY314 ; "type_checking_final_lazy")]
    #[test_case("SMP101_future.py", PythonVersion::PY310 ; "type_checking_final_future")]
    fn missing_final_by_version(fixture: &str, target_version: PythonVersion) {
        check(Rule::MissingFinal, fixture, target_version);
    }
}
