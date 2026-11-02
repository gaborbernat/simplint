use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::transformer::{self, Transformer};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Operator, Pattern, PythonVersion, Stmt, StringFlags};
use ruff_python_codegen::Generator;
use ruff_text_size::{Ranged, TextRange};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::lint::walker::{Scope, Walker};
use crate::rules::for_each_child_body;

pub fn expression<'a>(walker: &mut Walker<'_, 'a>, expr: &'a Expr) {
    let Expr::BinOp(ast::ExprBinOp { op: Operator::Add, .. }) = expr else {
        return;
    };
    // The `+` this one is the left side of reports the whole chain, unless what it adds is bytes.
    if let Some(Expr::BinOp(ast::ExprBinOp { left, op: Operator::Add, right, .. })) = walker.expression_parent()
        && std::ptr::eq(left.as_ref(), expr)
        && !right.is_bytes_literal_expr()
    {
        return;
    }
    let mut operands = Vec::new();
    flatten(expr, &mut operands);
    if !operands.iter().any(|operand| matches!(operand, Expr::StringLiteral(_) | Expr::FString(_)))
        || operands.iter().any(|operand| operand.is_bytes_literal_expr())
    {
        return;
    }
    let checker = walker.checker;
    let range = expr.range();
    // `+` calls an operand's own `__add__` or `__radd__`, and raises `TypeError` for one that is not text, where the
    // f-string formats any value: the two agree only when every operand is surely a `str`.
    let mut types = Types::new(walker, range);
    let texts = operands.iter().all(|operand| types.is_str(operand, types.depth));
    // A comment between the operands has no place in the f-string, so rewriting would drop it.
    let fix = fstring(checker, &operands, walker.in_fstring())
        .filter(|_| texts && !checker.has_comment_in(range))
        .map(|text| Fix::unsafe_edit(Edit::range_replacement(text, range)));
    checker.report_fix(
        Rule::StringConcatenation,
        range,
        "Build text with an f-string instead of `+`",
        fix,
        "Replace the concatenation with an f-string",
    );
}

fn flatten<'a>(expr: &'a Expr, operands: &mut Vec<&'a Expr>) {
    match expr {
        Expr::BinOp(ast::ExprBinOp { left, op: Operator::Add, right, .. }) => {
            flatten(left, operands);
            operands.push(right);
        }
        other => operands.push(other),
    }
}

// What the code shows about an operand's type, read off the scopes around the concatenation.
struct Types<'w, 'a> {
    walker: &'w Walker<'w, 'a>,
    // How many of the walker's scopes hold the concatenation in their body: a default value or decorator sits in the
    // scope around its def.
    depth: usize,
    // The names whose bindings are being checked, with the depth of the scope they resolve in. A binding that reads
    // the name it binds, as `text = text + "!"` does, holds text when the others do.
    resolving: Vec<(&'a str, usize)>,
}

impl<'w, 'a> Types<'w, 'a> {
    fn new(walker: &'w Walker<'w, 'a>, range: TextRange) -> Self {
        let depth = walker.scopes().iter().take_while(|scope| body_range(**scope).contains_range(range)).count();
        Self { walker, depth, resolving: Vec::new() }
    }

    // Whether `expr`, read in the innermost `depth` scopes, is surely a `str`.
    fn is_str(&mut self, expr: &'a Expr, depth: usize) -> bool {
        match expr {
            Expr::StringLiteral(_) | Expr::FString(_) => true,
            Expr::BinOp(ast::ExprBinOp { left, op: Operator::Add, right, .. }) => {
                self.is_str(left, depth) && self.is_str(right, depth)
            }
            Expr::If(ast::ExprIf { body, orelse, .. }) => self.is_str(body, depth) && self.is_str(orelse, depth),
            Expr::BoolOp(ast::ExprBoolOp { values, .. }) => values.iter().all(|value| self.is_str(value, depth)),
            Expr::Named(ast::ExprNamed { value, .. }) | Expr::Subscript(ast::ExprSubscript { value, .. }) => {
                self.is_str(value, depth)
            }
            Expr::Call(ast::ExprCall { func, .. }) => match func.as_ref() {
                Expr::Name(name) => STR_BUILTINS.contains(&name.id.as_str()) && self.lookup(&name.id, depth).is_none(),
                Expr::Attribute(ast::ExprAttribute { value, attr, .. }) => {
                    STR_METHODS.contains(&attr.as_str()) && self.is_str(value, depth)
                }
                _ => false,
            },
            Expr::Name(name) => self.name_is_str(&name.id, depth),
            _ => false,
        }
    }

    fn name_is_str(&mut self, name: &'a str, depth: usize) -> bool {
        let Some((bindings, depth)) = self.lookup(name, depth) else {
            return false;
        };
        if bindings.opaque {
            return false;
        }
        if bindings.annotated || self.resolving.contains(&(name, depth)) {
            return true;
        }
        self.resolving.push((name, depth));
        let texts = bindings.values.iter().all(|value| self.is_str(value, depth));
        self.resolving.pop();
        texts
    }

    // The bindings of `name` in the innermost of the `depth` scopes, then the module, that binds it, with that scope's
    // depth. A class body is skipped unless innermost, since its methods do not see it.
    fn lookup(&self, name: &str, depth: usize) -> Option<(Bindings<'a>, usize)> {
        let checker = self.walker.checker;
        let scopes = &self.walker.scopes()[..depth];
        scopes
            .iter()
            .enumerate()
            .rev()
            .filter(|(index, scope)| !matches!(scope, Scope::Class(_)) || index + 1 == depth)
            .map(|(index, scope)| (Bindings::of_scope(checker, *scope, name), index + 1))
            .chain(std::iter::once_with(|| (Bindings::of_body(checker, checker.body(), name), 0)))
            .find(|(bindings, _)| bindings.binds())
    }
}

// Builtins that always return a `str`.
const STR_BUILTINS: [&str; 8] = ["ascii", "bin", "chr", "format", "hex", "oct", "repr", "str"];

// Methods of `str` that return a `str`.
const STR_METHODS: [&str; 21] = [
    "capitalize",
    "casefold",
    "center",
    "expandtabs",
    "format",
    "format_map",
    "join",
    "ljust",
    "lower",
    "lstrip",
    "removeprefix",
    "removesuffix",
    "replace",
    "rjust",
    "rstrip",
    "strip",
    "swapcase",
    "title",
    "translate",
    "upper",
    "zfill",
];

fn body_range(scope: Scope) -> TextRange {
    let (body, end) = match scope {
        Scope::Function(function) => (&function.body, function.end()),
        Scope::Class(class) => (&class.body, class.end()),
        Scope::Lambda(lambda) => return lambda.body.range(),
    };
    TextRange::new(body.first().map_or(end, Ranged::start), end)
}

// How one scope binds a name: the values plain assignments give it, whether an annotation declares it a `str`, and
// whether anything else (a parameter, import, loop target, def, `global`) binds it to a value the code does not show.
#[derive(Default)]
struct Bindings<'a> {
    values: Vec<&'a Expr>,
    annotated: bool,
    opaque: bool,
}

impl<'a> Bindings<'a> {
    fn of_scope(checker: &Checker<'a>, scope: Scope<'a>, name: &str) -> Self {
        match scope {
            Scope::Function(function) => {
                let mut bindings = Self::of_parameters(checker, &function.parameters, name);
                bindings.collect(checker, &function.body, name);
                bindings
            }
            Scope::Class(class) => Self::of_body(checker, &class.body, name),
            Scope::Lambda(lambda) => {
                let mut bindings = lambda
                    .parameters
                    .as_deref()
                    .map(|parameters| Self::of_parameters(checker, parameters, name))
                    .unwrap_or_default();
                Collector { checker, name, bindings: &mut bindings }.visit_expr(&lambda.body);
                bindings
            }
        }
    }

    fn of_body(checker: &Checker<'a>, body: &'a [Stmt], name: &str) -> Self {
        let mut bindings = Self::default();
        bindings.collect(checker, body, name);
        bindings
    }

    fn of_parameters(checker: &Checker, parameters: &ast::Parameters, name: &str) -> Self {
        let mut bindings = Self::default();
        if let Some(parameter) = parameters.iter_non_variadic_params().find(|parameter| parameter.name() == name) {
            match parameter.annotation() {
                Some(annotation) if is_str_annotation(checker, annotation) => bindings.annotated = true,
                _ => bindings.opaque = true,
            }
        }
        bindings.opaque |= [&parameters.vararg, &parameters.kwarg]
            .into_iter()
            .flatten()
            .any(|parameter| parameter.name.as_str() == name);
        bindings
    }

    fn collect(&mut self, checker: &Checker<'a>, body: &'a [Stmt], name: &str) {
        Collector { checker, name, bindings: self }.visit_body(body);
    }

    fn binds(&self) -> bool {
        self.annotated || self.opaque || !self.values.is_empty()
    }
}

struct Collector<'c, 'a, 'b> {
    checker: &'c Checker<'a>,
    name: &'c str,
    bindings: &'b mut Bindings<'a>,
}

impl<'a> Visitor<'a> for Collector<'_, 'a, '_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        let name = self.name;
        let is_name = |target: &Expr| target.as_name_expr().is_some_and(|target| target.id == name);
        match stmt {
            // A nested def binds its own names; only its `global` or `nonlocal` reaches back here.
            Stmt::FunctionDef(ast::StmtFunctionDef { name: defined, body, .. })
            | Stmt::ClassDef(ast::StmtClassDef { name: defined, body, .. }) => {
                self.bindings.opaque |= defined.as_str() == name || declares(body, name);
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. }) => {
                for target in targets {
                    if is_name(target) {
                        self.bindings.values.push(value);
                    } else {
                        self.visit_expr(target);
                    }
                }
                self.visit_expr(value);
            }
            Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value, .. }) if is_name(target) => {
                if is_str_annotation(self.checker, annotation) {
                    self.bindings.annotated = true;
                } else if let Some(value) = value {
                    self.bindings.values.push(value);
                }
                if let Some(value) = value {
                    self.visit_expr(value);
                }
            }
            Stmt::AugAssign(ast::StmtAugAssign { target, op, value, .. }) if is_name(target) => {
                if *op == Operator::Add {
                    self.bindings.values.push(value);
                } else {
                    self.bindings.opaque = true;
                }
                self.visit_expr(value);
            }
            Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
                self.bindings.opaque |= names.iter().any(|declared| declared.as_str() == name);
            }
            Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
                self.bindings.opaque |= names.iter().any(|alias| {
                    let bound = alias.asname.as_ref().unwrap_or(&alias.name).as_str();
                    bound == "*" || bound.split('.').next() == Some(name)
                });
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(ast::ExprName { id, ctx, .. }) => {
                self.bindings.opaque |= id == self.name && *ctx != ExprContext::Load;
            }
            Expr::Lambda(_) => {}
            _ => visitor::walk_expr(self, expr),
        }
    }

    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        let ast::ExceptHandler::ExceptHandler(ast::ExceptHandlerExceptHandler { name: bound, .. }) = handler;
        self.bindings.opaque |= bound.as_ref().is_some_and(|bound| bound.as_str() == self.name);
        visitor::walk_except_handler(self, handler);
    }

    fn visit_pattern(&mut self, pattern: &'a Pattern) {
        let bound = match pattern {
            Pattern::MatchAs(ast::PatternMatchAs { name, .. })
            | Pattern::MatchStar(ast::PatternMatchStar { name, .. })
            | Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) => name.as_ref(),
            _ => None,
        };
        self.bindings.opaque |= bound.is_some_and(|bound| bound.as_str() == self.name);
        visitor::walk_pattern(self, pattern);
    }
}

// Whether a `global` or `nonlocal` anywhere in `body`, nested defs included, declares `name`.
fn declares(body: &[Stmt], name: &str) -> bool {
    body.iter().any(|stmt| match stmt {
        Stmt::Global(ast::StmtGlobal { names, .. }) | Stmt::Nonlocal(ast::StmtNonlocal { names, .. }) => {
            names.iter().any(|declared| declared.as_str() == name)
        }
        _ => {
            let mut found = false;
            for_each_child_body(stmt, &mut |child| found |= declares(child, name));
            found
        }
    })
}

// `str`, `"str"`, `Final[str]` or `Annotated[str, ...]`.
fn is_str_annotation(checker: &Checker, annotation: &Expr) -> bool {
    let semantic = checker.semantic();
    match annotation {
        Expr::Name(name) => name.id == "str" && !semantic.is_bound("str"),
        Expr::StringLiteral(string) => string.value.to_str() == "str",
        Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
            if semantic.match_typing_expr(value, "Final") {
                is_str_annotation(checker, slice)
            } else if semantic.match_typing_expr(value, "Annotated") {
                slice
                    .as_tuple_expr()
                    .and_then(|tuple| tuple.elts.first())
                    .is_some_and(|first| is_str_annotation(checker, first))
            } else {
                false
            }
        }
        _ => false,
    }
}

fn fstring(checker: &Checker, operands: &[&Expr], nested: bool) -> Option<String> {
    // Each string part's quote and the text between its quotes.
    let mut parts: Vec<(Quote, &str)> = Vec::new();
    // The f-string's text, with each interpolated expression, its source text and conversion still to place.
    let mut pieces: Vec<Result<String, (&Expr, &str, &str)>> = Vec::new();
    let mut interpolated = false;
    for operand in operands {
        match operand {
            // An implicit concatenation, which `ruff format` may join into one literal, joins here part by part.
            Expr::StringLiteral(string) => {
                for literal in &string.value {
                    let text = literal_text(checker, literal)?;
                    parts.push((literal.flags.quote_style(), text));
                    pieces.push(Ok(text.replace('{', "{{").replace('}', "}}")));
                }
            }
            Expr::FString(ast::ExprFString { value, .. }) => {
                for part in value {
                    match part {
                        ast::FStringPartRef::Literal(literal) => {
                            let text = literal_text(checker, literal)?;
                            parts.push((literal.flags.quote_style(), text));
                            pieces.push(Ok(text.replace('{', "{{").replace('}', "}}")));
                        }
                        ast::FStringPartRef::FString(fstring) => {
                            if fstring.flags.is_triple_quoted() || fstring.flags.prefix().is_raw() {
                                return None;
                            }
                            let text = checker.text(TextRange::new(
                                fstring.start() + fstring.flags.opener_len(),
                                fstring.end() - fstring.flags.closer_len(),
                            ));
                            parts.push((fstring.flags.quote_style(), text));
                            pieces.push(Ok(text.to_owned()));
                        }
                    }
                }
                interpolated = true;
            }
            other => {
                // `str(x)`, `repr(x)` and `ascii(x)` become the conversions `!s`, `!r` and `!a`, as Ruff's RUF010 asks.
                let (inner, conversion) = match *other {
                    Expr::Call(ast::ExprCall { func, arguments, .. })
                        if arguments.keywords.is_empty()
                            && arguments.args.len() == 1
                            && !arguments.args[0].is_starred_expr() =>
                    {
                        match func.as_name_expr().map(|name| name.id.as_str()) {
                            Some(builtin @ ("str" | "repr" | "ascii")) if !checker.semantic().is_bound(builtin) => {
                                (&arguments.args[0], &builtin[..1])
                            }
                            _ => (*other, ""),
                        }
                    }
                    _ => (*other, ""),
                };
                let text = checker.text(inner.range());
                if text.contains(FORBIDDEN_IN_EXPRESSION) {
                    return None;
                }
                pieces.push(Err((inner, text, conversion)));
                interpolated = true;
            }
        }
    }
    // A part in the other quote style fits only when its text holds no delimiter, escaped or not.
    // Before Python 3.12 a string inside an f-string's replacement field cannot use that f-string's quote either, so
    // there the parts keep the one quote they share and their expressions stay as written.
    let before_312 = checker.settings.from_ruff.target_version < PythonVersion::PY312;
    let as_written = nested && before_312;
    let preferred = checker.stylist.quote();
    let alternatives = if as_written { vec![] } else { vec![preferred, preferred.opposite()] };
    let delimiter = parts.first().map(|(quote, _)| *quote).into_iter().chain(alternatives).find(|delimiter| {
        parts.iter().all(|(quote, text)| quote == delimiter || !as_written && !text.contains(delimiter.as_char()))
    })?;
    let mut content = String::new();
    for piece in pieces {
        match piece {
            Ok(text) => content.push_str(&text),
            Err((expr, text, conversion)) => {
                content.push('{');
                content.push_str(&expression_text(checker, expr, text, delimiter, as_written)?);
                if !conversion.is_empty() {
                    content.push('!');
                    content.push_str(conversion);
                }
                content.push('}');
            }
        }
    }
    let prefix = if interpolated { "f" } else { "" };
    let text = if interpolated { content } else { content.replace("{{", "{").replace("}}", "}") };
    let delimiter = delimiter.as_char();
    Some(format!("{prefix}{delimiter}{text}{delimiter}"))
}

// The text of an expression as it goes into a replacement field of an f-string quoted with `delimiter`.
fn expression_text(checker: &Checker, expr: &Expr, text: &str, delimiter: Quote, as_written: bool) -> Option<String> {
    // Before Python 3.12 an f-string expression cannot reuse the string's own quote, so its string literals take the
    // other one, as `ruff format` leaves them once the f-string holds them.
    let before_312 = checker.settings.from_ruff.target_version < PythonVersion::PY312;
    let requote = before_312 && text.contains(delimiter.as_char());
    if requote && as_written {
        return None;
    }
    // An expression the layout spreads over lines goes on one, as it would in a line long enough.
    if !requote && !text.contains('\n') {
        return Some(text.to_owned());
    }
    let mut regenerated = expr.clone();
    if requote {
        Requote(delimiter.opposite()).visit_expr(&mut regenerated);
    }
    let text = Generator::from(checker.stylist).expr(&regenerated);
    let unfit = text.contains(FORBIDDEN_IN_EXPRESSION) || before_312 && text.contains(delimiter.as_char());
    (!unfit).then_some(text)
}

// A comment, a backslash, a format spec or conversion, or nested braces in an expression would not survive the move into
// a replacement field.
const FORBIDDEN_IN_EXPRESSION: [char; 6] = ['\\', '#', ':', '!', '{', '}'];

// A named escape such as `\N{BULLET}` keeps single braces in an f-string, where every other brace doubles.
fn literal_text<'a>(checker: &Checker<'a>, literal: &ast::StringLiteral) -> Option<&'a str> {
    let flags = literal.flags;
    let text = checker.text(literal.content_range());
    (!flags.is_triple_quoted() && !flags.prefix().is_raw() && !flags.prefix().is_unicode() && !text.contains("\\N{"))
        .then_some(text)
}

struct Requote(Quote);

impl Transformer for Requote {
    fn visit_expr(&self, expr: &mut Expr) {
        if let Expr::StringLiteral(ast::ExprStringLiteral { value, .. }) = expr {
            for literal in value.iter_mut() {
                literal.flags = literal.flags.with_quote_style(self.0);
            }
        }
        transformer::walk_expr(self, expr);
    }
}
