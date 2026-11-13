use ruff_diagnostics::Edit;
use ruff_formatter::LineWidth;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::{Mod, PySourceType, Stmt};
use ruff_python_formatter::{MagicTrailingComma, PyFormatOptions, QuoteStyle, format_module_ast, format_module_source};
use ruff_python_parser::{ParseOptions, Parsed, parse};
use ruff_python_trivia::{TriviaRanges, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::syntax::for_each_child_body;

// The width test of inlining: a header of `header` characters, joined on one line at the indentation of `depth` blocks,
// still fits the line width once the `name` in it becomes a value `wrapping` characters longer than the name's text.
pub(crate) fn fits_line(
    checker: &Checker,
    depth: usize,
    header: usize,
    name: usize,
    value: usize,
    wrapping: usize,
) -> bool {
    let indent = checker.settings.from_ruff.indent_width * depth;
    indent + header - name + value + wrapping <= checker.settings.from_ruff.line_width
}

// The value on one line, as `ruff format` joins it, without the parentheses it drops; `None` when the formatter keeps
// it split, as it does a value with a magic trailing comma. The quotes stay as written, since the replacement field's
// quote decides which quote the value may hold.
pub(crate) fn joined(checker: &Checker, text: &str) -> Option<String> {
    let options = formatting_options(checker).with_quote_style(QuoteStyle::Preserve);
    format_module_source(&format!("_ = ({text})\n"), options).ok().and_then(|printed| {
        printed.as_code().trim_end().strip_prefix("_ = ").filter(|line| !line.contains('\n')).map(str::to_owned)
    })
}

fn formatting_options(checker: &Checker) -> PyFormatOptions {
    let settings = &checker.settings.from_ruff;
    let trailing_comma =
        if settings.magic_trailing_comma { MagicTrailingComma::Respect } else { MagicTrailingComma::Ignore };
    // The widest line the formatter lays out, as the largest line width `ruff_formatter` accepts.
    PyFormatOptions::default()
        .with_target_version(settings.target_version)
        .with_magic_trailing_comma(trailing_comma)
        .with_line_width(LineWidth::try_from(320).expect("the widest line is a valid line width"))
}

// Whether `ruff format` keeps the statement's header on one line once the use is replaced. The formatter splits a
// statement wider than its line width, and it writes spacing and parentheses its own way, such as hugging `**`, so
// the header is formatted, at the widest line, with the replacement in it. Only the header counts: the block of a
// compound statement is left out, and is a `pass` when formatted.
pub(crate) fn fits(checker: &Checker, next: &Stmt, replace: &Edit, depth: usize) -> bool {
    let mut header_end = next.end();
    for_each_child_body(next, &mut |block| {
        if let Some(first) = block.first() {
            header_end = header_end.min(first.start());
        }
    });
    let compound = header_end < next.end();
    let header = TextRange::new(next.start(), header_end);
    // A comment in the header would stay on a line the layout may split, which the one-line measure does not model.
    if checker.has_comment_in(header) {
        return false;
    }
    // A read the walk reaches unconditionally sits in the header, so the replacement is in it.
    let content = replace.content().unwrap_or_default();
    let text = format!(
        "{}{content}{}",
        checker.text(TextRange::new(next.start(), replace.start())),
        checker.text(TextRange::new(replace.end(), header_end))
    );
    // The lines after the first keep the indentation of their block, which the formatter would not take at column zero.
    let indentation = leading_indentation(checker.source.full_line_str(next.start()));
    let text = text
        .trim_end()
        .split('\n')
        .enumerate()
        .map(|(index, line)| if index == 0 { line } else { line.strip_prefix(indentation).unwrap_or(line) })
        .collect::<Vec<_>>()
        .join("\n");
    let statement = if compound { format!("{text}\n    pass\n") } else { format!("{text}\n") };
    // The formatter would parse the statement the same way, so a statement that does not parse cannot be formatted.
    let Ok(parsed) = parse(&statement, ParseOptions::from(PySourceType::Python)) else {
        return false;
    };
    let indent = checker.settings.from_ruff.indent_width * depth;
    let limit = checker.settings.from_ruff.line_width;
    let respects_commas = checker.settings.from_ruff.magic_trailing_comma;
    if !text.contains('\n')
        && widest_format(&parsed, &statement, text.len(), respects_commas).is_some_and(|width| indent + width <= limit)
    {
        return true;
    }
    let trivia = TriviaRanges::from(parsed.tokens());
    let formatted = format_module_ast(&parsed, &trivia, &statement, formatting_options(checker));
    let printed = formatted.ok().and_then(|formatted| formatted.print().ok());
    // Each decorator and the statement are lines of their own, and a line the formatter indents is a split within one
    // of them, which the layout keeps, so every line must start at column zero and fit. A statement the formatter fails
    // on has no lines, which fit nowhere.
    let code = printed.as_ref().map_or("", |printed| printed.as_code().trim_end());
    let laid_out = if compound { code.rsplit_once('\n').map_or(code, |(head, _)| head) } else { code };
    !laid_out.is_empty()
        && laid_out.lines().all(|line| !line.starts_with(char::is_whitespace) && indent + line.chars().count() <= limit)
}

// The widest the formatter can print a one-line header, the first `header` bytes of `statement`, without formatting it.
// `None` when it may split the line: a magic trailing comma, a comma before a closing bracket, splits it at any width.
//
// The formatter prints a header that fits its line width on one line, token after token, so the printed width is at
// most the sum of what it prints for each token and each gap between two. A token keeps its width or shrinks, as a
// string loses escapes, a prefix or the quotes between joined parts, or grows by what `grown` allows.
//
// A gap gets no space where `is_tight` says so and at most one elsewhere, besides two parentheses where it reads an
// attribute off an integer, as `1 .real` becomes `(1).real`, unless it keeps the source's whitespace there, as in the
// debug text of an f-string's `=` field. A bound within the line width, which is never wider than the formatter's
// widest line, means the header prints on one line that fits, so the formatter need not run; the bound held for every
// candidate of the corpus and of CPython's `Lib` when checked against the formatter.
fn widest_format(parsed: &Parsed<Mod>, statement: &str, header: usize, respects_commas: bool) -> Option<usize> {
    let tokens: Vec<&Token> = parsed
        .tokens()
        .iter()
        .filter(|token| token.start().to_usize() < header && !token.kind().is_trivia() && !is_layout(token.kind()))
        .collect();
    let mut width = 0;
    for (at, token) in tokens.iter().enumerate() {
        width += statement[token.range()].chars().count() + grown(token.kind());
        let Some(next) = tokens.get(at + 1) else {
            continue;
        };
        let (kind, next_kind) = (token.kind(), next.kind());
        if respects_commas
            && kind == TokenKind::Comma
            && matches!(next_kind, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace)
        {
            return None;
        }
        let spaces = if kind == TokenKind::Int && next_kind == TokenKind::Dot {
            2
        } else {
            usize::from(!is_tight(kind, next_kind))
        };
        width += statement[TextRange::new(token.end(), next.start())].chars().count().max(spaces);
    }
    Some(width)
}

// How many characters the formatter may add for a token besides the space after it: the `0` a number may gain, and the
// parentheses it puts around the tuple a comma builds, as `first, = pair` becomes `(first,) = pair`, or around the
// power a unary operator applies to, as `-x ** 2` becomes `-(x**2)`.
fn grown(kind: TokenKind) -> usize {
    match kind {
        TokenKind::Float | TokenKind::Complex => 1,
        TokenKind::Comma | TokenKind::Minus | TokenKind::Plus | TokenKind::Tilde | TokenKind::Not => 2,
        _ => 0,
    }
}

fn is_layout(kind: TokenKind) -> bool {
    matches!(kind, TokenKind::Newline | TokenKind::Indent | TokenKind::Dedent | TokenKind::EndOfFile)
}

// Whether the formatter puts no space between two tokens: after an opening bracket or a `.`, before a closing bracket,
// a `,` or a `.`, and between a name, a string or a closing bracket and the `(` or `[` of a call or subscript.
fn is_tight(kind: TokenKind, next: TokenKind) -> bool {
    matches!(kind, TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace | TokenKind::Dot)
        || matches!(next, TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace | TokenKind::Comma | TokenKind::Dot)
        || (matches!(next, TokenKind::Lpar | TokenKind::Lsqb)
            && matches!(
                kind,
                TokenKind::Identifier
                    | TokenKind::Rpar
                    | TokenKind::Rsqb
                    | TokenKind::String
                    | TokenKind::FStringEnd
                    | TokenKind::TStringEnd
            ))
}
