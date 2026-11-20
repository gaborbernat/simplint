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
    let printed = format_module_source(&format!("_ = ({text})\n"), options).expect("a parenthesized expression parses");
    let line = printed.as_code().trim_end().strip_prefix("_ = ").expect("the formatter keeps the assignment target");
    (!line.contains('\n')).then(|| line.to_owned())
}

fn formatting_options(checker: &Checker) -> PyFormatOptions {
    let settings = &checker.settings.from_ruff;
    let trailing_comma =
        if settings.magic_trailing_comma { MagicTrailingComma::Respect } else { MagicTrailingComma::Ignore };
    // Use the maximum supported width to measure the header before applying the project's limit.
    PyFormatOptions::default()
        .with_target_version(settings.target_version)
        .with_magic_trailing_comma(trailing_comma)
        .with_line_width(LineWidth::try_from(320).expect("the widest line is a valid line width"))
}

// Measure the replacement after Ruff normalizes spacing and parentheses. A compound statement needs a dummy body
// so the formatter can parse its header.
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
    // A decorator can occupy its own line. An indented continuation means the header itself split.
    let code = printed.as_ref().map_or("", |printed| printed.as_code().trim_end());
    let laid_out = if compound { code.rsplit_once('\n').map_or(code, |(head, _)| head) } else { code };
    !laid_out.is_empty()
        && laid_out.lines().all(|line| !line.starts_with(char::is_whitespace) && indent + line.chars().count() <= limit)
}

// Skip the formatter when an upper bound for the printed header fits. Count possible token growth, inserted
// parentheses and spacing, including original whitespace that an f-string debug field can preserve.
// A magic trailing comma can force a split even when the header fits.
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
