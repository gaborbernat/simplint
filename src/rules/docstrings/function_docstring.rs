use std::cell::OnceCell;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Stmt};
use ruff_python_trivia::textwrap::dedent;
use ruff_python_trivia::{SuppressionKind as FormatSuppression, has_leading_content, has_trailing_content};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::comments::is_tool_directive;
use crate::rules::docstrings::rst::{SIGNATURE_SECTIONS, field_marker};
use crate::rules::docstrings::{Need, docstring_deletion, is_directive_comment};
use crate::rules::edits::lines_with_trailing_blanks;
use crate::rules::{docstring, indentation};

pub fn function_docstring(checker: &Checker) {
    // Code that reads docstrings off objects, such as a test asserting `func.__doc__` or a framework turning them into
    // help text or a schema, may read any of them, and which ones it reads does not show.
    if checker.project().reads_docstrings() {
        return;
    }
    let api = checker.state().public_api(checker);
    // A function or class an autodoc directive documents renders its docstring for its users, as public API does.
    let owners = checker
        .function_defs()
        .iter()
        .filter(|function| !api.covers(function.range, Need::Any))
        .map(|function| (function.range, function.body.as_slice(), function.decorator_list.as_slice(), "Function"))
        .chain(
            checker
                .class_defs()
                .iter()
                .filter(|class| !api.covers(class.range, Need::Any) && !reads_own_docstring(class))
                .map(|class| (class.range, class.body.as_slice(), class.decorator_list.as_slice(), "Class")),
        );
    let levels = OnceCell::new();
    let file_noqa = checker.state().has_file_noqa(checker);
    for (owner, owner_body, decorators, kind) in owners {
        let Some(literal) = docstring(owner_body) else {
            continue;
        };
        // A decorator receives the object and may read its docstring, as `click.command` does for help text, unless it
        // is one known to leave it unread.
        if decorators.iter().any(|decorator| !is_doc_blind(checker, &decorator.expression))
            || literal.value.to_str().lines().any(|line| line.trim_start().starts_with(">>>"))
        {
            continue;
        }
        let stmt = &owner_body[0];
        // The text as written, since an escape such as `\n` would otherwise end the comment line.
        let written: String = literal.value.iter().map(|part| checker.text(part.content_range())).collect();
        let lines = cleaned_lines(&written);
        // One short line rarely says more than the name; anything longer may state a contract the code does not show,
        // such as a tie-break rule, so its text stays as a comment.
        let restates = lines.len() <= 1 && lines.iter().all(|line| line.split_whitespace().count() <= 5);
        // The signature documents the parameters and results, so the comment leaves their field lists out.
        let documented = drop_fields(&lines);
        // Rewriting the docstring can leave a file-level `noqa` code unused, as with the lines it shortened.
        let (fix, title) = if file_noqa {
            (None, "Remove the docstring")
        } else if restates || documented.is_empty() {
            (removal(checker, stmt, owner_body).map(Fix::unsafe_edit), "Remove the docstring")
        } else {
            (
                comment_replacement(
                    checker,
                    owner,
                    owner_body,
                    &documented,
                    levels.get_or_init(|| block_levels(checker)),
                )
                .map(Fix::unsafe_edit),
                "Turn the docstring into a comment",
            )
        };
        checker.report_fix(
            Rule::FunctionDocstring,
            stmt.range(),
            format!("{kind} docstring on internal code; let the name, signature and types document it"),
            fix,
            title,
        );
    }
}

// The level of the block that starts after each indent or dedent token, by where the token starts, the module's being 1.
fn block_levels(checker: &Checker) -> Vec<(TextSize, usize)> {
    let mut level: usize = 1;
    let mut levels = Vec::new();
    for token in checker.parsed.tokens() {
        match token.kind() {
            TokenKind::Indent => level += 1,
            TokenKind::Dedent => level = level.saturating_sub(1),
            _ => continue,
        }
        levels.push((token.start(), level));
    }
    levels
}

// A class body that names `__doc__` reads its own docstring, which the compiler binds in the class namespace before the
// body runs. The headers of methods, lambdas and nested classes run in that body, as their decorators, defaults and
// bases do, while their bodies have a namespace of their own.
fn reads_own_docstring(class: &ast::StmtClassDef) -> bool {
    if docstring(&class.body).is_none() {
        return false;
    }
    let mut reads = ClassDocstringReads(false);
    reads.visit_body(&class.body);
    reads.0
}

struct ClassDocstringReads(bool);

impl<'a> Visitor<'a> for ClassDocstringReads {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, parameters, returns, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                self.visit_parameters(parameters);
                if let Some(returns) = returns {
                    self.visit_annotation(returns);
                }
            }
            Stmt::ClassDef(ast::StmtClassDef { decorator_list, arguments, .. }) => {
                for decorator in decorator_list {
                    self.visit_decorator(decorator);
                }
                if let Some(arguments) = arguments {
                    self.visit_arguments(arguments);
                }
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx: ExprContext::Load, .. }) => self.0 |= id.as_str() == "__doc__",
            Expr::Lambda(ast::ExprLambda { parameters, .. }) => {
                if let Some(parameters) = parameters {
                    self.visit_parameters(parameters);
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// Decorators that wrap or register the object without reading its docstring; copying it, as `functools.wraps` or
// `property` do, only hands it to `help()`. pytest shows a fixture's docstring in `pytest --fixtures`, a listing of the
// suite's own helpers like `help()`.
fn is_doc_blind(checker: &Checker, decorator: &Expr) -> bool {
    let decorator = decorator.as_call_expr().map_or(decorator, |call| &call.func);
    // `name.setter` on a property, whose name the module scope does not resolve.
    if decorator
        .as_attribute_expr()
        .is_some_and(|attribute| matches!(attribute.attr.as_str(), "setter" | "getter" | "deleter"))
    {
        return true;
    }
    let semantic = checker.semantic();
    if ["override", "overload", "final", "no_type_check"]
        .iter()
        .any(|member| semantic.match_typing_expr(decorator, member))
    {
        return true;
    }
    semantic.qualified_name(decorator).is_some_and(|name| {
        matches!(
            name.segments(),
            ["", "staticmethod" | "classmethod" | "property"]
                | ["abc", "abstractmethod"]
                | ["functools", "cache" | "lru_cache" | "cached_property" | "wraps" | "total_ordering"]
                | ["contextlib", "contextmanager" | "asynccontextmanager"]
                | ["dataclasses", "dataclass"]
                | ["enum", "unique"]
                | ["pytest", "fixture"]
                | ["pytest", "mark", ..]
        )
    })
}

fn removal(checker: &Checker, stmt: &Stmt, owner_body: &[Stmt]) -> Option<Edit> {
    // `ruff format` puts a body on lines of its own, so the fix must not depend on whether it already is. A pragma or
    // tool directive after the docstring, such as `# noqa: E501`, covers its line only and goes with it.
    if owner_body.len() == 1 {
        return Some(Edit::range_replacement("...".to_owned(), with_directive(checker, stmt)));
    }
    docstring_deletion(checker, owner_body)
}

// The comment goes where the docstring was, on lines of its own as `ruff format` would leave the body, wrapped to the
// width its lines get once formatted, so the result does not depend on the layout.
fn comment_replacement(
    checker: &Checker,
    owner: TextRange,
    owner_body: &[Stmt],
    lines: &[String],
    levels: &[(TextSize, usize)],
) -> Option<Edit> {
    let source = checker.source;
    let stmt = &owner_body[0];
    let on_header = has_leading_content(stmt.start(), source);
    let indent = if on_header {
        format!("{}{}", indentation(source, owner), checker.stylist.indentation().as_str())
    } else {
        indentation(source, stmt).to_owned()
    };
    // The dedents closing the blocks before the owner sit at its start.
    let before = &levels[..levels.partition_point(|(at, _)| *at <= owner.start())];
    let level = before.last().map_or(1, |(_, level)| *level);
    let from_ruff = &checker.settings.from_ruff;
    let room = from_ruff.comment_width.saturating_sub(from_ruff.indent_width * level + 2);
    let comments: Vec<String> = comment_lines(lines, room)
        .into_iter()
        .map(|line| if line.is_empty() { "#".to_owned() } else { format!("# {line}") })
        .collect();
    // A line such as `type: ignore ...` or `fmt: off`, or one holding `# noqa` in its text, would turn into a directive
    // for a tool. Ruff's line length rules let a docstring line of a single word run long, but not a comment line
    // unless it ends in a URL. Where `ruff format` leaves the code alone, an indentation Ruff's E114 rejects for
    // comments stays.
    if comments.iter().any(|comment| {
        comment.match_indices('#').any(|(start, _)| is_tool_directive(&comment[start..]))
            || comment.chars().count() > room + 2
                && !comment.split_whitespace().last().is_some_and(|word| word.contains("://"))
    }) || indent.chars().count() % from_ruff.indent_width.max(1) != 0 && is_left_alone(checker, stmt)
    {
        return None;
    }
    let newline = format!("{}{indent}", checker.stylist.line_ending().as_str());
    let block = comments.join(&newline);
    let start = if on_header { TextSize::of(source[..stmt.start().to_usize()].trim_end()) } else { stmt.start() };
    let prefix = if on_header { newline.as_str() } else { "" };
    let (end, suffix) = match owner_body.get(1) {
        None => (with_directive(checker, stmt).end(), format!("{newline}...")),
        // A statement after a `;` moves to a line of its own, unless a comment between them would go with it.
        Some(next) if has_trailing_content(stmt.end(), source) => {
            if checker.has_comment_in(TextRange::new(stmt.end(), next.start())) {
                return None;
            }
            (next.start(), newline.clone())
        }
        // Up to the next statement's line, as deleting the docstring would, so a fix removing that statement in the
        // same pass touches this one and waits, rather than leaving a body of comments only.
        Some(_) => {
            let comment = checker
                .trailing_comment(stmt)
                .filter(|comment| !is_directive_comment(checker, *comment))
                .map_or_else(String::new, |comment| format!("  {}", checker.text(comment)));
            (
                lines_with_trailing_blanks(source, stmt.range()).end(),
                format!("{comment}{}", checker.stylist.line_ending().as_str()),
            )
        }
    };
    Some(Edit::range_replacement(format!("{prefix}{block}{suffix}"), TextRange::new(start, end)))
}

// Whether `ruff format` leaves the statement as it is: its settings exclude the file, the statement ends in
// `# fmt: skip`, or it follows an own-line `# fmt: off` with no `# fmt: on` between.
fn is_left_alone(checker: &Checker, stmt: &Stmt) -> bool {
    let source = checker.source;
    let suppression = |comment: &TextRange| FormatSuppression::from_comment(checker.text(*comment));
    let mut off = false;
    for comment in checker.comment_ranges().iter().take_while(|comment| comment.start() < stmt.start()) {
        if has_leading_content(comment.start(), source) {
            continue;
        }
        match suppression(comment) {
            Some(FormatSuppression::Off) => off = true,
            Some(FormatSuppression::On) => off = false,
            _ => {}
        }
    }
    off || !checker.settings.from_ruff.formatted
        || checker.trailing_comment(stmt).and_then(|comment| suppression(&comment)) == Some(FormatSuppression::Skip)
}

fn with_directive(checker: &Checker, stmt: &Stmt) -> TextRange {
    let directive = checker.trailing_comment(stmt).filter(|comment| is_directive_comment(checker, *comment));
    stmt.range().cover(directive.unwrap_or(stmt.range()))
}

// The docstring's text as `inspect.cleandoc` gives it, without trailing whitespace and without the underlines of rst
// and numpydoc headings, which as comments would read as section dividers.
fn cleaned_lines(text: &str) -> Vec<String> {
    let text = text.replace('\t', "        ");
    let (first, rest) = text.split_once('\n').unwrap_or((&text, ""));
    let mut lines: Vec<String> = std::iter::once(first.trim().to_owned())
        .chain(dedent(rest).lines().map(|line| line.trim_end().to_owned()))
        .filter(|line| !is_underline(line))
        .collect();
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    let leading = lines.iter().take_while(|line| line.is_empty()).count();
    lines.drain(..leading);
    lines
}

fn is_underline(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.len() >= 3 && trimmed.chars().all(|char| matches!(char, '-' | '=' | '~' | '^' | '*' | '+' | '#' | '_'))
}

// Drops the reST fields and the sections that document the signature, with their bodies, and the blank lines that
// would be left separating nothing.
fn drop_fields(lines: &[String]) -> Vec<String> {
    let mut kept: Vec<String> = Vec::new();
    let mut index = 0;
    while index < lines.len() {
        if let Some(end) = field_block_end(lines, index) {
            index = end;
            continue;
        }
        let line = &lines[index];
        if !(line.is_empty() && kept.last().is_none_or(String::is_empty)) {
            kept.push(line.clone());
        }
        index += 1;
    }
    while kept.last().is_some_and(String::is_empty) {
        kept.pop();
    }
    kept
}

// The index after the field or section starting at `start`, when it starts one.
fn field_block_end(lines: &[String], start: usize) -> Option<usize> {
    let line = &lines[start];
    let trimmed = line.trim();
    let lead = indent_width(line);
    if field_marker(trimmed).is_some() {
        return Some(indented_end(lines, start, lead));
    }
    let (name, colon) = match trimmed.strip_suffix(':') {
        Some(name) => (name, true),
        None => (trimmed, false),
    };
    if !SIGNATURE_SECTIONS.contains(&name) {
        return None;
    }
    if colon {
        return Some(indented_end(lines, start, lead));
    }
    // A numpydoc section lists its names at the header's own indentation, so its body runs up to a blank line or the
    // next section.
    let body = lines[start + 1..]
        .iter()
        .position(|line| line.trim().is_empty() || SIGNATURE_SECTIONS.contains(&line.trim()))
        .unwrap_or(lines.len() - start - 1);
    Some(start + 1 + body)
}

// The index after the last line indented past `lead` that follows `start`, blank lines in between included.
fn indented_end(lines: &[String], start: usize, lead: usize) -> usize {
    let mut end = start + 1;
    for (offset, line) in lines[start + 1..].iter().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        if indent_width(line) <= lead {
            break;
        }
        end = start + 2 + offset;
    }
    end
}

fn indent_width(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

// Lines that fit are kept as written, so lists and indented blocks keep their shape. What a long line wraps over joins
// the next line when that line goes on with the same sentence, rather than leaving a word on a line of its own.
fn comment_lines(lines: &[String], room: usize) -> Vec<String> {
    let mut comments = Vec::new();
    let mut carry = String::new();
    for (index, line) in lines.iter().enumerate() {
        let lead = &line[..line.len() - line.trim_start().len()];
        let line = if carry.is_empty() { line.clone() } else { format!("{lead}{carry} {}", line.trim_start()) };
        let mut parts = wrapped(&line, room);
        carry = if parts.len() > 1 && lines.get(index + 1).is_some_and(|next| continues(next, lead)) {
            parts.pop().unwrap_or_default().trim_start().to_owned()
        } else {
            String::new()
        };
        comments.extend(parts);
    }
    comments
}

// A line at the same indentation starting in lowercase goes on with a sentence; one opening with a field name such as
// `name:` or `name (int):` starts an item of its own.
fn continues(line: &str, lead: &str) -> bool {
    line.strip_prefix(lead).is_some_and(|text| {
        text.starts_with(char::is_lowercase) && !text.split_whitespace().take(2).any(|word| word.ends_with(':'))
    })
}

// Splits a line too long for `room` at spaces, keeping its indentation on each part; a word longer than `room`, such as
// a URL, keeps a line of its own.
fn wrapped(line: &str, room: usize) -> Vec<String> {
    if line.chars().count() <= room {
        return vec![line.to_owned()];
    }
    let lead = &line[..line.len() - line.trim_start().len()];
    let mut parts = Vec::new();
    let mut current = String::new();
    for word in line.split_whitespace() {
        if !current.is_empty() && current.chars().count() + 1 + word.chars().count() > room {
            parts.push(std::mem::take(&mut current));
        }
        if current.is_empty() {
            current.push_str(lead);
            current.push_str(word);
        } else {
            current.push(' ');
            current.push_str(word);
        }
    }
    parts.push(current);
    parts
}
