// What inlining or moving a value takes, shared by the rules that do so, such as `single-use-variable` and
// `early-assignment`: the walks over the scopes a body sits in and the names it reads and binds, the classes of
// literals, the text that replaces a use, and the width the statement then needs.
use std::borrow::Cow;

use ruff_diagnostics::Edit;
use ruff_formatter::LineWidth;
use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::str::Quote;
use ruff_python_ast::token::{Token, TokenKind};
use ruff_python_ast::visitor::transformer::{self, Transformer};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Mod, Pattern, PySourceType, Stmt, StringFlags, UnaryOp};
use ruff_python_codegen::Generator;
use ruff_python_formatter::{MagicTrailingComma, PyFormatOptions, QuoteStyle, format_module_ast, format_module_source};
use ruff_python_parser::{ParseOptions, Parsed, parse, parse_expression};
use ruff_python_trivia::{TriviaRanges, leading_indentation};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::lint::checker::Checker;
use crate::rules::edits::escape_braces;
use crate::syntax::{for_each_body, for_each_child_body, last_segment};

// Calls `visit` with each statement list under `body`, the scope it sits in and how many indented blocks enclose it.
// `enter` gives the scope of the statements inside a compound statement.
pub(super) fn for_each_scoped_body<'a, S: Copy>(
    body: &'a [Stmt],
    scope: S,
    depth: usize,
    enter: &impl Fn(S, &'a Stmt) -> S,
    visit: &mut impl FnMut(&'a [Stmt], S, usize),
) {
    visit(body, scope, depth);
    for stmt in body {
        let inner = enter(scope, stmt);
        // A `case` body sits a block inside the `match` block.
        let blocks = if stmt.is_match_stmt() { 2 } else { 1 };
        for_each_child_body(stmt, &mut |child| for_each_scoped_body(child, inner, depth + blocks, enter, visit));
    }
}

// The value as the use takes it, parenthesised when `parenthesize` is set and the source does not already parenthesise
// it. The value's range is the one its parentheses sit in, so the source text is compared with the bare value.
pub(super) fn inlined_text(checker: &Checker, value: &Expr, value_range: TextRange, parenthesize: bool) -> String {
    let text = checker.text(value_range);
    if parenthesize && text == checker.text(value.range()) { format!("({text})") } else { text.to_owned() }
}

// The names a function declares `global` or `nonlocal`, in its nested blocks and functions too.
pub(super) fn declared_outer(body: &[Stmt]) -> FxHashSet<&str> {
    let mut names = FxHashSet::default();
    for_each_body(body, &mut |statements, _| {
        for stmt in statements {
            if let Stmt::Global(ast::StmtGlobal { names: declared, .. })
            | Stmt::Nonlocal(ast::StmtNonlocal { names: declared, .. }) = stmt
            {
                names.extend(declared.iter().map(ast::Identifier::as_str));
            }
        }
    });
    names
}

// The name a test starts with, through `not`, comparisons and `and`/`or`. `bare` is set while the name is the test
// itself rather than an operand of it.
pub(super) fn leading_name(test: &Expr, bare: bool) -> Option<(&ast::ExprName, bool)> {
    match test {
        Expr::Name(name) => Some((name, bare)),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => leading_name(operand, false),
        Expr::Compare(ast::ExprCompare { operands, .. }) => leading_name(operands.first()?, false),
        Expr::BoolOp(ast::ExprBoolOp { values, .. }) => leading_name(values.first()?, false),
        _ => None,
    }
}

// The width test of inlining: a header of `header` characters, joined on one line at the indentation of `depth` blocks,
// still fits the line width once the `name` in it becomes a value `wrapping` characters longer than the name's text.
pub(super) fn fits_line(
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

// The expression's text with every string in it in the other quote than `delimiter`, for a replacement field before
// Python 3.12 that cannot hold `delimiter`. `None` when a string cannot change quote, or the text still holds
// `delimiter` or a backslash.
pub(super) fn requoted_text(checker: &Checker, expr: &Expr, delimiter: Quote) -> Option<String> {
    if any_over_expr(expr, &|sub: &Expr| unquotable(checker, sub)) {
        return None;
    }
    let mut regenerated = expr.clone();
    Requote(delimiter.opposite()).visit_expr(&mut regenerated);
    let text = Generator::from(checker.stylist).expr(&regenerated);
    (!text.contains(delimiter.as_char()) && !text.contains('\\')).then_some(text)
}

// A triple-quoted, raw or `\N{` string keeps its form only in its own quotes, so it cannot change quote. Neither can an
// f-string whose replacement field holds a string: every string of the value takes the other quote, so the field would
// hold the quote its f-string is written in, which Python before 3.12 does not allow.
fn unquotable(checker: &Checker, expr: &Expr) -> bool {
    match expr {
        Expr::StringLiteral(_) => fstring_body_parts(checker, expr).is_none(),
        Expr::FString(ast::ExprFString { value, .. }) => {
            fstring_body_parts(checker, expr).is_none()
                || value.elements().any(|element| match element {
                    ast::InterpolatedStringElement::Interpolation(interpolation) => {
                        any_over_expr(&interpolation.expression, &|sub: &Expr| {
                            matches!(sub, Expr::StringLiteral(_) | Expr::FString(_))
                        })
                    }
                    ast::InterpolatedStringElement::Literal(_) => false,
                })
        }
        _ => false,
    }
}

fn literal_part<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<(Quote, Cow<'a, str>)> {
    Some((literal.flags.quote_style(), Cow::Owned(escape_braces(literal_text(checker, literal)?))))
}

// A named escape such as `\N{BULLET}` keeps single braces in an f-string, where every other brace doubles.
fn literal_text<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<&'a str> {
    let flags = literal.flags;
    let text = checker.text(literal.content_range());
    (!flags.is_triple_quoted() && !flags.prefix().is_raw() && !flags.prefix().is_unicode() && !text.contains("\\N{"))
        .then_some(text)
}

// The parts of a string or f-string value as an f-string body takes them, each with its quote: a literal's text with its
// braces doubled, and an f-string's text between its quotes as it stands. `None` when a part cannot move into an
// f-string body, which is a literal `literal_text` refuses or a raw or triple-quoted f-string.
pub(super) fn fstring_body_parts<'a>(checker: &Checker<'a>, value: &Expr) -> Option<Vec<(Quote, Cow<'a, str>)>> {
    match value {
        Expr::StringLiteral(string) => string.value.iter().map(|literal| literal_part(checker, literal)).collect(),
        Expr::FString(ast::ExprFString { value, .. }) => value
            .iter()
            .map(|part| match part {
                ast::FStringPartRef::Literal(literal) => literal_part(checker, literal),
                ast::FStringPartRef::FString(fstring) => {
                    if fstring.flags.is_triple_quoted() || fstring.flags.prefix().is_raw() {
                        return None;
                    }
                    let content = TextRange::new(
                        fstring.start() + fstring.flags.opener_len(),
                        fstring.end() - fstring.flags.closer_len(),
                    );
                    Some((fstring.flags.quote_style(), Cow::Borrowed(checker.text(content))))
                }
            })
            .collect(),
        _ => None,
    }
}

struct Requote(Quote);

impl Transformer for Requote {
    fn visit_expr(&self, expr: &mut Expr) {
        match expr {
            Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) => {
                for literal in value.iter_mut() {
                    literal.flags = literal.flags.with_quote_style(self.0);
                }
            }
            Expr::FString(ast::ExprFString { value, .. }) => {
                for part in value.iter_mut() {
                    match part {
                        ast::FStringPartMut::Literal(literal) => literal.flags = literal.flags.with_quote_style(self.0),
                        ast::FStringPartMut::FString(fstring) => fstring.flags = fstring.flags.with_quote_style(self.0),
                    }
                }
            }
            _ => {}
        }
        transformer::walk_expr(self, expr);
    }
}

// A literal whose value is the same wherever it is evaluated and that cannot raise: a constant or a display of them.
// A dict key or a set element is hashed when it is built, so a list or dict there is not a constant of that kind.
pub(super) fn is_constant(expr: &Expr) -> bool {
    match expr {
        Expr::NumberLiteral(_)
        | Expr::StringLiteral(_)
        | Expr::BytesLiteral(_)
        | Expr::BooleanLiteral(_)
        | Expr::NoneLiteral(_)
        | Expr::EllipsisLiteral(_) => true,
        // `-` and `+` take a number, so the operand must be one.
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::Not, operand, .. }) => is_constant(operand),
        Expr::UnaryOp(ast::ExprUnaryOp { op: UnaryOp::UAdd | UnaryOp::USub, operand, .. }) => {
            operand.is_number_literal_expr()
        }
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().all(is_constant)
        }
        Expr::Set(ast::ExprSet { elts, .. }) => elts.iter().all(is_hashable),
        Expr::Dict(ast::ExprDict { items, .. }) => {
            items.iter().all(|item| item.key.as_ref().is_some_and(is_hashable) && is_constant(&item.value))
        }
        _ => false,
    }
}

pub(super) fn is_hashable(expr: &Expr) -> bool {
    match expr {
        Expr::Tuple(ast::ExprTuple { elts, .. }) => elts.iter().all(is_hashable),
        Expr::List(_) | Expr::Set(_) | Expr::Dict(_) => false,
        _ => is_constant(expr),
    }
}

// The value on one line, as `ruff format` joins it, without the parentheses it drops; `None` when the formatter keeps it
// split, as it does a value with a magic trailing comma. The quotes stay as written, since the replacement field's quote
// decides which quote the value may hold.
pub(super) fn joined(checker: &Checker, text: &str) -> Option<String> {
    let options = formatting_options(checker).with_quote_style(QuoteStyle::Preserve);
    let printed = format_module_source(&format!("_ = ({text})\n"), options).ok()?;
    printed.as_code().trim_end().strip_prefix("_ = ").filter(|line| !line.contains('\n')).map(str::to_owned)
}

fn formatting_options(checker: &Checker) -> PyFormatOptions {
    let settings = &checker.settings.from_ruff;
    let trailing_comma =
        if settings.magic_trailing_comma { MagicTrailingComma::Respect } else { MagicTrailingComma::Ignore };
    PyFormatOptions::default()
        .with_target_version(settings.target_version)
        .with_magic_trailing_comma(trailing_comma)
        .with_line_width(LineWidth::try_from(MAX_LINE_WIDTH).expect("the widest line is a valid line width"))
}

// The widest line the formatter lays out, as the largest line width `ruff_formatter` accepts.
const MAX_LINE_WIDTH: u16 = 320;

// Whether `ruff format` keeps the statement's header on one line once the use is replaced. The formatter splits a
// statement wider than its line width, and it writes spacing and parentheses its own way, such as hugging `**`, so
// the header is formatted, at the widest line, with the replacement in it. Only the header counts: the block of a
// compound statement is left out, and is a `pass` when formatted.
pub(super) fn fits(checker: &Checker, next: &Stmt, replace: &Edit, depth: usize) -> bool {
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
    let indent = checker.settings.from_ruff.indent_width * depth;
    let limit = checker.settings.from_ruff.line_width;
    // The formatter would parse the statement the same way, so a statement that does not parse cannot be formatted.
    let Ok(parsed) = parse(&statement, ParseOptions::from(PySourceType::Python)) else {
        return false;
    };
    let respects_commas = checker.settings.from_ruff.magic_trailing_comma;
    if !text.contains('\n')
        && widest_format(&parsed, &statement, text.len(), respects_commas).is_some_and(|width| indent + width <= limit)
    {
        return true;
    }
    let trivia = TriviaRanges::from(parsed.tokens());
    let formatted = format_module_ast(&parsed, &trivia, &statement, formatting_options(checker));
    let printed = formatted.ok().and_then(|formatted| formatted.print().ok());
    // Each decorator and the statement are lines of their own, and a line the formatter indents is a split within one of
    // them, which the layout keeps, so every line must start at column zero and fit. A statement the formatter fails on
    // has no lines, which fit nowhere.
    let code = printed.as_ref().map_or("", |printed| printed.as_code().trim_end());
    let laid_out = if compound { code.rsplit_once('\n').map_or(code, |(head, _)| head) } else { code };
    !laid_out.is_empty()
        && laid_out.lines().all(|line| !line.starts_with(char::is_whitespace) && indent + line.chars().count() <= limit)
}

// The widest the formatter can print a one-line header, the first `header` bytes of `statement`, without formatting
// it. `None` when it may split the line: a magic trailing comma, a comma before a closing bracket, splits it at any
// width.
//
// The formatter prints a header that fits its line width on one line, token after token, so the printed width is at
// most the sum of what it prints for each token and each gap between two. A token keeps its width or shrinks, as a
// string loses escapes, a prefix or the quotes between joined parts, or grows by what `grown` allows. A gap gets no
// space where `is_tight` says so and at most one elsewhere, besides two parentheses where it reads an attribute off an
// integer, as `1 .real` becomes `(1).real`, unless it keeps the source's whitespace there, as in the debug text of an
// f-string's `=` field. A bound within the line width, which is never wider than the formatter's widest line, means the
// header prints on one line that fits, so the formatter need not run; the bound held for every candidate of the
// corpus and of CPython's `Lib` when checked against the formatter.
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

// Built-ins that read the locals by name, so a variable they read cannot be inlined away.
const LOCALS_READERS: [&str; 4] = ["eval", "exec", "locals", "vars"];

// Every mention of each name in a function, found in one walk of its body, so a function with many assignments is
// walked once rather than once per assignment.
pub(super) struct Names<'a> {
    pub(super) mentions: FxHashMap<&'a str, Vec<(TextRange, ExprContext)>>,
    // Names the function binds in a way the walk does not follow: a `def`, a `class`, an import, an `except` target or
    // a pattern capture.
    pub(super) rebound: FxHashSet<&'a str>,
    pub(super) declared_outer: FxHashSet<&'a str>,
    // Names a string annotation or a `ForwardRef` reads. The walk does not see them, and they are read whenever the
    // annotation is evaluated, so no name they read is inlined.
    pub(super) string_reads: FxHashSet<String>,
    in_annotation: bool,
    // Whether the function reads its locals by name, which keeps every name alive.
    pub(super) reads_locals: bool,
}

impl<'a> Names<'a> {
    pub(super) fn of(body: &'a [Stmt]) -> Self {
        let mut names = Self {
            mentions: FxHashMap::default(),
            rebound: FxHashSet::default(),
            declared_outer: declared_outer(body),
            string_reads: FxHashSet::default(),
            in_annotation: false,
            reads_locals: false,
        };
        names.visit_body(body);
        names
    }
}

impl<'a> Visitor<'a> for Names<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(definition) => {
                self.rebound.insert(definition.name.as_str());
            }
            Stmt::ClassDef(class) => {
                self.rebound.insert(class.name.as_str());
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.rebound.extend(names.iter().map(root_binding));
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name, .. }) = handler;
        self.rebound.extend(name.as_ref().map(ast::Identifier::as_str));
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let captured = match pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name, .. }) => name.as_ref().map(ast::Identifier::as_str),
            Pattern::MatchMapping(ast::PatternMatchMapping { rest, .. }) => rest.as_ref().map(ast::Identifier::as_str),
            _ => None,
        };
        self.rebound.extend(captured);
        visitor::walk_pattern(self, pattern);
    }

    fn visit_annotation(&mut self, expr: &'a Expr) {
        let outer = std::mem::replace(&mut self.in_annotation, true);
        visitor::walk_annotation(self, expr);
        self.in_annotation = outer;
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => {
                self.mentions.entry(name.id.as_str()).or_default().push((name.range, name.ctx));
                self.reads_locals |= LOCALS_READERS.contains(&name.id.as_str());
            }
            Expr::StringLiteral(string) if self.in_annotation => self.read_forward_reference(string),
            Expr::Call(ast::ExprCall { func, arguments, .. }) if last_segment(func) == Some("ForwardRef") => {
                for argument in &arguments.args {
                    if let Expr::StringLiteral(string) = argument {
                        self.read_forward_reference(string);
                    }
                }
            }
            _ => {}
        }
        visitor::walk_expr(self, expr);
    }
}

impl Names<'_> {
    // A string annotation is code that Ruff reads as an expression, so the names it spells are reads.
    fn read_forward_reference(&mut self, string: &ast::ExprStringLiteral) {
        let Ok(parsed) = parse_expression(string.value.to_str()) else {
            return;
        };
        let mut collector = NameCollector::default();
        collector.visit_expr(parsed.expr());
        self.string_reads.extend(collector.names);
    }
}

// The names an expression reads, as their own text spells them.
#[derive(Default)]
struct NameCollector {
    names: Vec<String>,
}

impl Visitor<'_> for NameCollector {
    fn visit_expr(&mut self, expr: &Expr) {
        if let Expr::Name(name) = expr {
            self.names.push(name.id.to_string());
        }
        visitor::walk_expr(self, expr);
    }
}

// The name an `import` binds in the scope: its `as` name, or the first part of a dotted module name, which `import a.b`
// binds as `a`.
fn root_binding(alias: &ast::Alias) -> &str {
    alias
        .asname
        .as_ref()
        .map_or_else(|| alias.name.as_str().split('.').next().unwrap_or_default(), |name| name.as_str())
}

// `function` is the innermost `def` around a body; only statements inside a function are checked.
pub(super) fn enter_function<'a>(
    function: Option<&'a ast::StmtFunctionDef>,
    stmt: &'a Stmt,
) -> Option<&'a ast::StmtFunctionDef> {
    match stmt {
        Stmt::FunctionDef(definition) => Some(definition),
        Stmt::ClassDef(_) => None,
        _ => function,
    }
}
