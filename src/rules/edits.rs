use ruff_diagnostics::Edit;
use ruff_python_ast::helpers::is_docstring_stmt;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Stmt, StringFlags};
use ruff_python_stdlib::sys::is_known_standard_library;
use ruff_python_trivia::leading_indentation;
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextLen, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::rules::{indentation, is_alone_on_lines};
use crate::semantic::future_import;
use crate::syntax::previous_line_start;

pub fn delete_statement(source: &str, stmt: &Stmt, body: &[Stmt]) -> Edit {
    if body.len() == 1 {
        return Edit::range_replacement("pass".to_owned(), stmt.range());
    }
    Edit::range_deletion(source.full_lines_range(stmt.range()))
}

// The text deleting a statement removes: its lines when it has them to itself, with the blank lines after them when
// `trailing_blanks` is set. A statement sharing its line goes from the `;` that joins it to the statement before it, as
// in `run(); x = 1`, or up to the `next` statement, as in `"""Doc."""; run()`, unless a comment in between would go
// with it; `ruff format` puts joined statements on lines of their own either way.
pub fn statement_deletion(
    checker: &Checker,
    range: TextRange,
    next: Option<TextRange>,
    trailing_blanks: bool,
) -> Option<TextRange> {
    let source = checker.source;
    if is_alone_on_lines(source, range) {
        return Some(if trailing_blanks {
            lines_with_trailing_blanks(source, range)
        } else {
            lines_with_shared_blanks(source, range)
        });
    }
    let line_start = source.line_start(range.start());
    let before = source[TextRange::new(line_start, range.start())].trim_end();
    let after = &source[TextRange::new(range.end(), source.line_end(range.end()))];
    if before.ends_with(';') && after.trim().is_empty() {
        return Some(TextRange::new(line_start + before.text_len() - TextSize::from(1), range.end()));
    }
    let up_to_next = TextRange::new(range.start(), next?.start());
    (!checker.has_comment_in(up_to_next)).then_some(up_to_next)
}

// The lines of a statement, with the blank lines after it that `ruff format` would not keep once it is gone. The
// formatter keeps two blank lines at module level and one in a block, so a statement between blank lines leaves a
// surplus it would close, which `# fmt: off` code keeps.
fn lines_with_shared_blanks(source: &str, range: TextRange) -> TextRange {
    let lines = source.full_lines_range(range);
    let limit = if leading_indentation(source.full_line_str(range.start())).is_empty() { 2 } else { 1 };
    let mut before: usize = 0;
    let mut start = lines.start();
    while start > TextSize::default() && source.full_line_str(previous_line_start(source, start)).trim().is_empty() {
        start = previous_line_start(source, start);
        before += 1;
    }
    let mut after: usize = 0;
    let mut end = lines.end();
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
        after += 1;
    }
    let mut surplus_end = lines.end();
    for _ in 0..(before + after).saturating_sub(limit).min(after) {
        surplus_end = source.full_line_end(surplus_end);
    }
    TextRange::new(lines.start(), surplus_end)
}

pub fn lines_with_trailing_blanks(source: &str, range: TextRange) -> TextRange {
    let mut end = source.full_line_end(range.end());
    while end.to_usize() < source.len() && source.full_line_str(end).trim().is_empty() {
        end = source.full_line_end(end);
    }
    let mut start = source.line_start(range.start());
    // At the end of the file the blank lines above go too: after a module left with comments only, `ruff format` keeps
    // them, and Ruff reports blank lines ending the file.
    if end.to_usize() == source.len() {
        while start > TextSize::default() && source.full_line_str(previous_line_start(source, start)).trim().is_empty()
        {
            start = previous_line_start(source, start);
        }
    }
    TextRange::new(start, end)
}

// The full lines of a statement with its decorators and the comment lines directly above it, which go wherever it goes.
pub fn statement_block(checker: &Checker, stmt: &Stmt) -> TextRange {
    let source = checker.source;
    let start = match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, .. })
        | Stmt::ClassDef(ast::StmtClassDef { decorator_list, .. }) => {
            decorator_list.first().map_or(stmt.start(), Ranged::start)
        }
        _ => stmt.start(),
    };
    let mut line_start = source.line_start(start);
    // A line inside a string may start with `#` too, so only a comment token above the statement counts. A deeper
    // comment ends the block above it, as the trailing note of a method does.
    let indent = indentation(source, stmt);
    while let Some(comment) =
        checker.comment_above(TextRange::empty(line_start)).filter(|comment| indentation(source, *comment) == indent)
    {
        line_start = source.line_start(comment.start());
    }
    TextRange::new(line_start, source.full_line_end(stmt.end()))
}

// The keyword with the comma that separates it from a neighbor, so the rest of the call keeps its layout. A sole
// argument goes with its trailing comma, up to the closing parenthesis.
pub fn keyword_removal(checker: &Checker, call: &ast::ExprCall, keyword: &ast::Keyword) -> Option<TextRange> {
    if checker.has_comment_in(call.range()) {
        return None;
    }
    let mut arguments: Vec<TextRange> = call
        .arguments
        .args
        .iter()
        .map(|argument| checker.parenthesized_range(argument, (&call.arguments).into()))
        .chain(call.arguments.keywords.iter().map(Ranged::range))
        .collect();
    arguments.sort_by_key(Ranged::start);
    let index = arguments.iter().position(|range| *range == keyword.range())?;
    let range = keyword.range();
    let previous = index.checked_sub(1).map(|previous| arguments[previous]);
    Some(match (previous, arguments.get(index + 1)) {
        (_, Some(next)) => TextRange::new(range.start(), next.start()),
        (Some(previous), None) => TextRange::new(previous.end(), range.end()),
        (None, None) => TextRange::new(range.start(), call.range().end() - TextSize::from(1)),
    })
}

// Rewrites the edits that change a name read in an f-string field with a `=` debug specifier, so the field prints what
// it printed before. The field prints its source text, `x=`, then the value of the expression, so the rewrite keeps that
// text as written and moves the edits into a plain field: `{x=}` becomes `x={x!r}`, and a conversion or format spec
// carries over. `None` when an edit lands where that cannot hold, such as in a t-string field, which keeps its source
// text in the template, or in a field nested in another one.
pub fn keep_debug_text(checker: &Checker, edits: Vec<Edit>) -> Option<Vec<Edit>> {
    let finder = checker.state().debug_fields(checker);
    let mut groups: Vec<Vec<Edit>> = vec![Vec::new(); finder.fields.len()];
    let mut rewritten = Vec::new();
    for edit in edits {
        if finder.blocked.iter().any(|range| range.contains_range(edit.range())) {
            return None;
        }
        let mut containing =
            finder.fields.iter().enumerate().filter(|(_, field)| field.range.contains_range(edit.range()));
        match (containing.next(), containing.next()) {
            (None, _) => rewritten.push(edit),
            (Some((index, _)), None) => groups[index].push(edit),
            (Some(_), Some(_)) => return None,
        }
    }
    for (field, edits) in finder.fields.iter().zip(groups) {
        if !edits.is_empty() {
            rewritten.push(Edit::range_replacement(debug_field(checker.source, field, &edits)?, field.range));
        }
    }
    Some(rewritten)
}

// The text with each brace doubled, as an f-string spells a literal brace.
pub fn escape_braces(text: &str) -> String {
    text.replace('{', "{{").replace('}', "}}")
}

// The text of a `=` debug field with its edits applied: the debug text, escaped to a literal, then a plain field.
fn debug_field(source: &str, field: &DebugField, edits: &[Edit]) -> Option<String> {
    let debug = field.debug.as_str();
    if debug.contains(['\n', '\r', '\\']) {
        return None;
    }
    let quote = field.flags.quote_style().as_char();
    // A raw f-string keeps the backslash that an escaped quote would need, so the quote cannot go in its literal text.
    if field.flags.prefix().is_raw() && debug.contains(quote) {
        return None;
    }
    let start = usize::from(field.range.start()) + 1;
    let text_end = start + debug.len();
    let equals = start + debug.rfind('=')?;
    let end = usize::from(field.range.end()) - 1;
    let (mut expression_edits, mut remainder_edits): (Vec<&Edit>, Vec<&Edit>) =
        edits.iter().partition(|edit| usize::from(edit.start()) >= start && usize::from(edit.end()) <= equals);
    let expression = splice(source, start, equals, &mut expression_edits)?;
    let expression = expression.trim_end();
    let remainder = splice(source, text_end, end, &mut remainder_edits)?;
    let conversion = if remainder.is_empty() { "!r" } else { &remainder };
    let space = if expression.starts_with('{') { " " } else { "" };
    let literal = escape_braces(debug).replace(quote, &format!("\\{quote}"));
    Some(format!("{literal}{{{space}{expression}{conversion}}}"))
}

// The source between `from` and `to` with the edits inside it applied. `None` when an edit lies outside the span.
fn splice(source: &str, from: usize, to: usize, edits: &mut [&Edit]) -> Option<String> {
    edits.sort_by_key(|edit| usize::from(edit.start()));
    let mut text = String::new();
    let mut cursor = from;
    for edit in edits.iter() {
        text.push_str(source.get(cursor..usize::from(edit.start()))?);
        text.push_str(edit.content().unwrap_or_default());
        cursor = usize::from(edit.end());
    }
    text.push_str(source.get(cursor..to)?);
    Some(text)
}

// The name a fix uses for `member` of `module`, or for `module` itself, and the edit that adds the import when the file
// lacks it. `None` when another binding holds the name at `at`, the place the fix writes it, which leaves the fix off.
pub fn ensure_import(
    checker: &Checker,
    module: &str,
    member: Option<&str>,
    at: TextRange,
) -> Option<(String, Option<Edit>)> {
    let binding = member.unwrap_or(module);
    if is_bound_around(checker, binding, at) {
        return None;
    }
    if has_import(checker.body(), module, member) {
        return Some((binding.to_owned(), None));
    }
    if checker.semantic().is_bound(binding) {
        return None;
    }
    Some((binding.to_owned(), Some(import_insertion(checker, module, member)?)))
}

// Whether a scope around `at` binds the name, which hides the module-level binding or the builtin a fix writes there.
pub fn is_bound_around(checker: &Checker, name: &str, at: TextRange) -> bool {
    checker.semantic().is_bound_around(name, at, checker.state().comprehensions(checker))
}

// Whether a top-level import already brings the module, or the member, in under its own name.
fn has_import(body: &[Stmt], module: &str, member: Option<&str>) -> bool {
    body.iter().any(|stmt| match (stmt, member) {
        (Stmt::Import(ast::StmtImport { names, .. }), None) => {
            names.iter().any(|alias| alias.name.as_str() == module && alias.asname.is_none())
        }
        (Stmt::ImportFrom(ast::StmtImportFrom { module: Some(from), names, level: 0, .. }), Some(member)) => {
            from.as_str() == module && names.iter().any(|alias| alias.name.as_str() == member && alias.asname.is_none())
        }
        _ => false,
    })
}

// The import goes where Ruff's isort keeps it: in the standard library section of the leading import block, plain
// `import x` lines before `from x import y` lines, each group in module order. A new section gets a blank line after it.
fn import_insertion(checker: &Checker, module: &str, member: Option<&str>) -> Option<Edit> {
    let body = checker.body();
    let source = checker.source;
    let eol = checker.stylist.line_ending().as_str();
    let text = match member {
        Some(member) => format!("from {module} import {member}{eol}"),
        None => format!("import {module}{eol}"),
    };
    let key = (member.is_some(), module.to_ascii_lowercase());
    let preamble = body.iter().take_while(|stmt| future_import(stmt).is_some() || is_docstring_stmt(stmt)).count();
    let block = body[preamble..].iter().take_while(|stmt| matches!(stmt, Stmt::Import(_) | Stmt::ImportFrom(_)));
    let minor = checker.settings.from_ruff.target_version.minor;
    let stdlib: Vec<(&Stmt, (bool, String))> = block
        .filter_map(|stmt| match stmt {
            Stmt::Import(ast::StmtImport { names, .. }) => names
                .iter()
                .all(|alias| is_known_standard_library(minor, alias.name.split('.').next().unwrap_or_default()))
                .then(|| (stmt, (false, names[0].name.to_ascii_lowercase()))),
            Stmt::ImportFrom(ast::StmtImportFrom { module: Some(module), level: 0, .. }) => {
                is_known_standard_library(minor, module.split('.').next().unwrap_or_default())
                    .then(|| (stmt, (true, module.to_ascii_lowercase())))
            }
            _ => None,
        })
        .collect();
    let (line, insertion) = if let Some((next, _)) = stdlib.iter().find(|(_, sort)| *sort > key) {
        (above_comments(checker, next), text)
    } else if let Some((last, _)) = stdlib.last() {
        let line_end = source.full_line_end(last.end());
        let rest = source[TextRange::new(last.end(), line_end)].trim();
        // Code after the import on its line would run before the new import lands below it.
        if !rest.is_empty() && !rest.starts_with('#') {
            return None;
        }
        (line_end, text)
    } else {
        // The statement the import goes before follows the preamble, so the body has one there.
        (above_comments(checker, &body[preamble]), format!("{text}{eol}"))
    };
    // A line that starts inside the preamble's last statement would put the import above a docstring sharing its line,
    // which then stops being `__doc__`, or above a `from __future__` import, which is a syntax error.
    if preamble > 0 && line < body[preamble - 1].end() {
        return None;
    }
    Some(Edit::insertion(insertion, line))
}

// The line an import goes on above `stmt`: above the comment lines directly over it, unless they start the file, where a
// shebang, encoding or licence header stays put.
fn above_comments(checker: &Checker, stmt: &Stmt) -> TextSize {
    let source = checker.source;
    let mut line_start = source.line_start(stmt.start());
    while checker.comment_above(TextRange::empty(line_start)).is_some() {
        line_start = previous_line_start(source, line_start);
    }
    if line_start > source.bom_start_offset() { line_start } else { source.line_start(stmt.start()) }
}

// The `=` debug fields of a module, each with its quote and the text of its debug specifier.
#[derive(Default)]
pub(super) struct DebugFields<'a> {
    fields: Vec<DebugField<'a>>,
    // The debug fields inside a t-string, which the rewrite cannot change.
    blocked: Vec<TextRange>,
}

struct DebugField<'a> {
    range: TextRange,
    debug: &'a ast::DebugText,
    flags: ast::FStringFlags,
}

impl<'a> Visitor<'a> for DebugFields<'a> {
    fn visit_f_string(&mut self, f_string: &'a ast::FString) {
        self.fields.extend(f_string.elements.iter().filter_map(|element| debug_field_of(element, f_string.flags)));
        visitor::walk_f_string(self, f_string);
    }

    fn visit_t_string(&mut self, t_string: &'a ast::TString) {
        self.blocked.extend(t_string.elements.iter().filter_map(|element| match element {
            ast::InterpolatedStringElement::Interpolation(interpolation) => {
                interpolation.debug_text.as_ref().map(|_| interpolation.range)
            }
            ast::InterpolatedStringElement::Literal(_) => None,
        }));
        visitor::walk_t_string(self, t_string);
    }
}

fn debug_field_of(element: &ast::InterpolatedStringElement, flags: ast::FStringFlags) -> Option<DebugField<'_>> {
    let ast::InterpolatedStringElement::Interpolation(interpolation) = element else {
        return None;
    };
    Some(DebugField { range: interpolation.range, debug: interpolation.debug_text.as_ref()?, flags })
}
