use ruff_diagnostics::{Edit, Fix};
use ruff_linter::registry::Rule as RuffRule;
use ruff_python_ast::helpers::{any_over_expr, is_dunder};
use ruff_python_ast::name::QualifiedName;
use ruff_python_ast::str::Quote;
use ruff_python_ast::token::TokenKind;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, OperatorPrecedence, PythonVersion, Stmt, StringFlags};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashMap;

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::edits::keep_debug_text;
use crate::rules::edits::statement_deletion;
use crate::rules::logic::inline::{
    Names, enter_function, fits, for_each_scoped_body, fstring_body_parts, inlined_text, joined, leading_name,
    requoted_text,
};
use crate::rules::{is_alone_on_lines, needs_parentheses};
use crate::syntax::{is_name, is_repeating_expr, last_segment};

pub fn single_use_variable(checker: &Checker) {
    // A function's names are walked once, and only when one of its blocks assigns a single name.
    let mut names: FxHashMap<TextRange, Names> = FxHashMap::default();
    for_each_scoped_body(checker.body(), None, 0, &enter_function, &mut |body, function, depth| {
        let Some(function) = function else {
            return;
        };
        for pair in body.windows(2) {
            if let [Stmt::Assign(assign), next] = pair
                && let [Expr::Name(target)] = assign.targets.as_slice()
            {
                let names = names.entry(function.range).or_insert_with(|| Names::of(&function.body));
                check_pair(checker, names, (assign, target), next, depth);
            }
        }
    });
}

// `assign` binds `target`, its only target.
fn check_pair<'a>(
    checker: &Checker<'a>,
    names: &Names<'a>,
    (assign, target): (&'a ast::StmtAssign, &'a ast::ExprName),
    next: &'a Stmt,
    depth: usize,
) {
    let name = target.id.as_str();
    let Some((use_range, reading)) = single_read(checker, names, (assign, target), next) else {
        return;
    };
    let Some((range, replacement)) = inlined_replacement(checker, assign, next, (use_range, &reading)) else {
        return;
    };
    if ruff_writes_same(checker, (assign, target), next, &replacement) {
        return;
    }
    let deletion = statement_deletion(checker, assign.range, Some(next.range()), false)
        .expect("no comment sits between the assignment and the statement after it");
    let deletion = Edit::range_deletion(deletion);
    // An empty value, such as `''` spliced into an f-string, leaves nothing to write, which only a deletion expresses.
    let edit =
        if replacement.is_empty() { Edit::range_deletion(range) } else { Edit::range_replacement(replacement, range) };
    let edits = keep_debug_text(checker, vec![edit]);
    let fix = match edits {
        Some(edits) => {
            // A read in a `=` debug field rewrites the whole field, so the width is measured on the field it becomes.
            let Some(replace) = edits.first() else { return };
            if !fits(checker, next, replace, depth) {
                return;
            }
            Some(if reading.effects { Fix::unsafe_edits(deletion, edits) } else { Fix::safe_edits(deletion, edits) })
        }
        // A debug field that cannot be rewritten keeps the report, as the variable is still used once.
        None => None,
    };
    checker.report_fix(
        Rule::SingleUseVariable,
        assign.range,
        format!("`{name}` is used once, right below; inline its value"),
        fix,
        format!("Inline `{name}`"),
    );
}

// The one read of the assigned name that the statement after the assignment makes, with its range, when inlining the
// value there keeps the value's evaluation in order.
fn single_read<'a>(
    checker: &Checker<'a>,
    names: &Names<'a>,
    (assign, target): (&'a ast::StmtAssign, &'a ast::ExprName),
    next: &'a Stmt,
) -> Option<(TextRange, Reading<'a>)> {
    let name = target.id.as_str();
    let value = assign.value.as_ref();
    // A `while` test runs again on every pass, and `_` is a placeholder rather than a name to read.
    if name == "_"
        || is_dunder(name)
        || matches!(value, Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Await(_) | Expr::Named(_))
        || matches!(next, Stmt::While(_))
        || checker.has_comment_in(TextRange::new(assign.start(), next.start()))
        || names.declared_outer.contains(name)
        || leads_if_test(next, name)
        || returns_or_yields(checker, next, name)
        || raises_with_name(next, name)
        || names.rebound.contains(name)
        || names.string_reads.contains(name)
        || names.reads_locals
    {
        return None;
    }
    let mut others =
        names.mentions.get(name).into_iter().flatten().copied().filter(|(range, _)| *range != target.range);
    let (Some((use_range, ExprContext::Load)), None) = (others.next(), others.next()) else {
        return None;
    };
    if !next.range().contains_range(use_range) {
        return None;
    }
    let reading = Reading::of(next, use_range);
    // Inlining moves the generator into the call, which then drops it, so its `finally` runs at the call's end.
    if reading.read != Some(false) || reading.consumes {
        return None;
    }
    if reading.default_parameter.is_some_and(|parameter| reports_default(checker, value, parameter)) {
        return None;
    }
    // A conditional or a string literal nested in a replacement field reads worse than the statements it replaces.
    if !reading.fstring_quotes.is_empty()
        && !matches!(value, Expr::StringLiteral(_) | Expr::FString(_))
        && any_over_expr(value, &|sub: &Expr| matches!(sub, Expr::StringLiteral(_) | Expr::FString(_) | Expr::If(_)))
    {
        return None;
    }
    // Ruff's C419 writes a list comprehension passed to `all` or `any` as a generator, which stops at the first element
    // that decides the result, so the work of the elements past it would not run.
    if reading.short_circuits && matches!(value, Expr::ListComp(_)) && any_over_expr(value, &is_effect) {
        return None;
    }
    Some((use_range, reading))
}

// The range the read replaces and the text that replaces it: the assigned value as the statement takes it,
// parenthesised where it needs that. A bare field of an f-string is replaced whole, by the value's text spliced in.
fn inlined_replacement(
    checker: &Checker,
    assign: &ast::StmtAssign,
    next: &Stmt,
    (use_range, reading): (TextRange, &Reading<'_>),
) -> Option<(TextRange, String)> {
    let value = assign.value.as_ref();
    let before_312 = checker.settings.from_ruff.target_version < PythonVersion::PY312;
    if let Some(field) = reading.bare_field
        && let Some(text) = spliced_text(checker, value, reading.field_fstring, before_312)
    {
        return Some((field, text));
    }
    // The formatter drops the parentheses around an assigned value, so the use does not get them either, and
    // `inlined_text` puts them back where the value needs them. A value over several lines keeps them outside brackets,
    // where its line breaks would end the statement.
    let value_range = if checker.text(value.range()).contains(['\n', '\r']) && !in_brackets(checker, next, use_range) {
        checker.parenthesized_range(value, assign.into())
    } else {
        value.range()
    };
    let value_text = checker.text(value_range);
    // A replacement field cannot hold a backslash or a line break before Python 3.12 (PEP 701), so a value the source
    // spreads over lines goes in on one line, as `ruff format` joins it once the assignment is gone.
    let field_before_312 = before_312 && !reading.fstring_quotes.is_empty();
    if field_before_312 && value_text.contains('\\') {
        return None;
    }
    let joined =
        if field_before_312 && value_text.contains(['\n', '\r']) { Some(joined(checker, value_text)?) } else { None };
    // Before Python 3.9 a decorator is a dotted name, optionally called, so the value must be one.
    if checker.settings.from_ruff.target_version < PythonVersion::PY39
        && reading.decorator_head
        && !is_dotted_name(value)
    {
        return None;
    }
    // A whole argument, return or assigned value needs no parentheses, though a tuple still does.
    let parenthesize = needs_parentheses_at(value, reading.slot) && (!reading.whole || value.is_tuple_expr());
    let mut replacement = field_value(
        checker,
        (value, value_range, joined.as_deref()),
        parenthesize,
        &reading.fstring_quotes,
        before_312,
    )?;
    // A value that starts with `{` would make the field's `{` an escaped `{{`, so the value gets a space.
    if reading.field_start
        && replacement.starts_with('{')
        && checker.source[..use_range.start().to_usize()].ends_with('{')
    {
        replacement = format!(" {replacement} ");
    }
    Some((use_range, replacement))
}

// The text of a string value that takes the place of a bare field of the f-string around it, with the f-string's
// braces. `None` where the field's f-string cannot hold the text: a raw or triple-quoted part, a quote of the
// enclosing f-string in the text, or a backslash before Python 3.12.
fn spliced_text(checker: &Checker, value: &Expr, outer: Option<ast::FStringFlags>, before_312: bool) -> Option<String> {
    let outer = outer?;
    if outer.prefix().is_raw() {
        return None;
    }
    let text: String = fstring_body_parts(checker, value)?.into_iter().map(|(_, text)| text).collect();
    let splices_wrongly = text.contains(outer.quote_style().as_char()) || (before_312 && text.contains('\\'));
    (!splices_wrongly).then_some(text)
}

// The precedence the value needs to stay one operand where the read sits, with the parentheses `needs_parentheses`
// asks for where the read is in no operator, as an argument or a call.
fn needs_parentheses_at(value: &Expr, slot: Option<OperatorPrecedence>) -> bool {
    match slot {
        Some(required) => {
            OperatorPrecedence::from_expr(value) < required
                || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
        }
        None => needs_parentheses(value),
    }
}

// Whether Ruff's B006 or B008 reports the value once it is the default of `parameter`. Both skip a parameter whose
// annotation is immutable.
fn reports_default(checker: &Checker, value: &Expr, parameter: &ast::ParameterWithDefault) -> bool {
    let from_ruff = &checker.settings.from_ruff;
    let b006 = from_ruff.ruff_runs(RuffRule::MutableArgumentDefault);
    let b008 = from_ruff.ruff_runs(RuffRule::FunctionCallInDefaultArgument);
    if !(b006 || b008) {
        return false;
    }
    let extend_immutable_calls: Vec<QualifiedName> =
        from_ruff.extend_immutable_calls.iter().map(|target| QualifiedName::from_dotted_name(target)).collect();
    if parameter
        .annotation()
        .is_some_and(|annotation| checker.semantic().has_immutable_annotation(annotation, &extend_immutable_calls))
    {
        return false;
    }
    let mut calls = DefaultCalls { checker, extend_immutable_calls: &extend_immutable_calls, reported: false };
    if b008 {
        calls.visit_expr(value);
    }
    calls.reported || (b006 && is_b006_mutable(checker, value))
}

// The values B006 reports as a mutable default. In preview, it also reports a generator and a tuple that holds one of
// the mutable values.
fn is_b006_mutable(checker: &Checker, value: &Expr) -> bool {
    match value {
        Expr::Generator(_) if checker.settings.from_ruff.preview => true,
        Expr::Tuple(ast::ExprTuple { elts, .. }) if checker.settings.from_ruff.preview => {
            elts.iter().any(|elt| is_b006_mutable(checker, elt))
        }
        _ => checker.semantic().is_mutable_value(value),
    }
}

struct DefaultCalls<'a, 'b> {
    checker: &'a Checker<'b>,
    extend_immutable_calls: &'a [QualifiedName<'a>],
    reported: bool,
}

impl<'a> Visitor<'a> for DefaultCalls<'_, '_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Lambda(_) => {}
            Expr::Call(ast::ExprCall { func, .. })
                if self.checker.semantic().is_b008_call(func, self.extend_immutable_calls) =>
            {
                self.reported = true;
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// The precedence the operand that holds the read must have in its operator, `None` when the read is not a direct operand
// of an operator or test, or the operator is not one that asks for a precedence here.
fn operand_precedence(expr: &Expr, reference: TextRange) -> Option<OperatorPrecedence> {
    let direct = |operand: &Expr| operand.range() == reference;
    match expr {
        Expr::BoolOp(ast::ExprBoolOp { op, values, .. }) if values.iter().any(direct) => Some(match op {
            ast::BoolOp::Or => OperatorPrecedence::And,
            ast::BoolOp::And => OperatorPrecedence::Not,
        }),
        Expr::If(ast::ExprIf { test, body, .. }) if direct(test) || direct(body) => Some(OperatorPrecedence::Or),
        Expr::Compare(ast::ExprCompare { operands, .. }) if operands.iter().any(direct) => {
            Some(OperatorPrecedence::BitOr)
        }
        Expr::BinOp(ast::ExprBinOp { left, op, right, .. }) => {
            // The left operand keeps the operator's precedence and the right one takes the next level up, as the
            // operators associate to the left, except `**`, which takes its left operand from `await` and its right one
            // from a unary operator.
            let (left_level, right_level) = match op {
                ast::Operator::BitOr => (OperatorPrecedence::BitOr, OperatorPrecedence::BitXor),
                ast::Operator::BitXor => (OperatorPrecedence::BitXor, OperatorPrecedence::BitAnd),
                ast::Operator::BitAnd => (OperatorPrecedence::BitAnd, OperatorPrecedence::LeftRightShift),
                ast::Operator::LShift | ast::Operator::RShift => {
                    (OperatorPrecedence::LeftRightShift, OperatorPrecedence::AddSub)
                }
                ast::Operator::Add | ast::Operator::Sub => {
                    (OperatorPrecedence::AddSub, OperatorPrecedence::MulDivRemain)
                }
                ast::Operator::Pow => (OperatorPrecedence::Await, OperatorPrecedence::PosNegBitNot),
                ast::Operator::Mult
                | ast::Operator::MatMult
                | ast::Operator::Div
                | ast::Operator::Mod
                | ast::Operator::FloorDiv => (OperatorPrecedence::MulDivRemain, OperatorPrecedence::PosNegBitNot),
            };
            if direct(left) { Some(left_level) } else { direct(right).then_some(right_level) }
        }
        Expr::UnaryOp(ast::ExprUnaryOp { op, operand, .. }) if direct(operand) => Some(match op {
            ast::UnaryOp::Not => OperatorPrecedence::Not,
            ast::UnaryOp::Invert | ast::UnaryOp::UAdd | ast::UnaryOp::USub => OperatorPrecedence::PosNegBitNot,
        }),
        _ => None,
    }
}

// The value as it goes into the statement. Before Python 3.12 a replacement field cannot hold the quote of the f-string
// around it, so a value that holds it takes the other quote in its strings, which needs every such f-string to share one
// quote.
fn field_value(
    checker: &Checker,
    (value, value_range, joined): (&Expr, TextRange, Option<&str>),
    parenthesize: bool,
    quotes: &[Quote],
    before_312: bool,
) -> Option<String> {
    let value_text = joined.unwrap_or_else(|| checker.text(value_range));
    if !(before_312 && quotes.iter().any(|quote| value_text.contains(quote.as_char()))) {
        return Some(match joined {
            Some(text) if parenthesize => format!("({text})"),
            Some(text) => text.to_owned(),
            None => inlined_text(checker, value, value_range, parenthesize),
        });
    }
    let quote = quotes.first().filter(|first| quotes.iter().all(|quote| quote == *first))?;
    let text = requoted_text(checker, value, *quote)?;
    Some(if parenthesize { format!("({text})") } else { text })
}

fn in_brackets(checker: &Checker, next: &Stmt, use_range: TextRange) -> bool {
    let tokens = checker.parsed.tokens().in_range(TextRange::new(next.start(), use_range.start()));
    let opened =
        tokens.iter().filter(|token| matches!(token.kind(), TokenKind::Lpar | TokenKind::Lsqb | TokenKind::Lbrace));
    let closed =
        tokens.iter().filter(|token| matches!(token.kind(), TokenKind::Rpar | TokenKind::Rsqb | TokenKind::Rbrace));
    opened.count() > closed.count()
}

// Ruff's RUF070 reports the same pair as a `yield from` of the assigned name, and writes the same code when the value
// goes in as written and both statements stand alone on their lines, so the pair is left to it. Only the preview rule
// does, so the pair is left to it only when preview is on and the rule runs.
fn ruff_writes_same(
    checker: &Checker,
    (assign, target): (&ast::StmtAssign, &ast::ExprName),
    next: &Stmt,
    replacement: &str,
) -> bool {
    if !checker.settings.from_ruff.preview
        || !checker.settings.from_ruff.ruff_runs(RuffRule::UnnecessaryAssignBeforeYield)
    {
        return false;
    }
    let Stmt::Expr(ast::StmtExpr { value, .. }) = next else {
        return false;
    };
    let Expr::YieldFrom(ast::ExprYieldFrom { value: yielded, .. }) = value.as_ref() else {
        return false;
    };
    let source = checker.source;
    // Ruff keeps the source between `=` and the value only when it is one space or none, and writes the value as it is.
    // It also writes one space after the keyword, which this rule keeps only when the source has one.
    let between = checker.text(TextRange::new(target.range().end(), assign.value.start()));
    is_name(yielded, target.id.as_str())
        && checker.text(TextRange::new(next.start(), yielded.start())) == "yield from "
        && (between.ends_with('=') || between.ends_with("= "))
        && replacement == checker.text(assign.value.range())
        && is_alone_on_lines(source, assign.range())
        && is_alone_on_lines(source, next.range())
        && !checker.has_comment_in(TextRange::new(next.start(), source.full_line_end(next.end())))
        && source.full_line_end(assign.end()) == source.line_start(next.start())
}

// Ruff's EM101 and EM102 ask for the message of an exception to be a variable, so the `raise` keeps it.
fn raises_with_name(next: &Stmt, name: &str) -> bool {
    let Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) = next else {
        return false;
    };
    exc.as_call_expr().is_some_and(|call| call.arguments.args.iter().any(|argument| is_name(argument, name)))
}

// `return name` is left to Ruff's RET504 while that rule runs, which covers the return case. This rule would also add an
// inferred return annotation to a literal value, which Ruff does not write. `yield name` is never reported, as before.
fn returns_or_yields(checker: &Checker, next: &Stmt, name: &str) -> bool {
    let value = match next {
        Stmt::Return(ast::StmtReturn { value, .. })
            if checker.settings.from_ruff.ruff_runs(RuffRule::UnnecessaryAssign) =>
        {
            value.as_deref()
        }
        Stmt::Expr(ast::StmtExpr { value, .. }) => match value.as_ref() {
            Expr::Yield(ast::ExprYield { value, .. }) => value.as_deref(),
            _ => None,
        },
        _ => None,
    };
    value.is_some_and(|value| is_name(value, name))
}

// An `if` whose test starts with the name is SMP502's, which writes the value into the test itself.
fn leads_if_test(next: &Stmt, name: &str) -> bool {
    let Stmt::If(ast::StmtIf { test, .. }) = next else {
        return false;
    };
    leading_name(test, true).is_some_and(|(found, _)| found.id.as_str() == name)
}

// The value of a decorator before Python 3.9, which the grammar limits to a dotted name.
fn is_dotted_name(expr: &Expr) -> bool {
    match expr {
        Expr::Name(_) => true,
        Expr::Attribute(ast::ExprAttribute { value, .. }) => is_dotted_name(value),
        _ => false,
    }
}

// How the next statement reads the name, walked in evaluation order. `read` is set once the walk reaches the read, to
// whether it runs only in some passes or branches: in a nested block, a lambda or a comprehension. A read the walk
// never reaches stays unset, and is not reported. `effects` is set when something before the read may have side
// effects, so inlining would reorder them.
#[expect(clippy::struct_excessive_bools, reason = "each flag marks one place the read sits in")]
struct Reading<'a> {
    reference: TextRange,
    // The statements the walk is in, the next statement counting as one.
    statements: usize,
    // The lambdas and comprehensions the walk is in.
    repeats: usize,
    // The branches the walk is in, where a part of the code may be skipped or evaluated later.
    branches: usize,
    // Whether the read is conditional, set once the walk reaches it.
    read: Option<bool>,
    effects: bool,
    // The read is a whole argument, keyword value, return value or assigned value, where no parentheses are needed.
    whole: bool,
    // The read starts its f-string field, so a value that starts with `{` would follow the field's `{`.
    field_start: bool,
    // The whole field the read fills, `{name}`, when it has no conversion, format spec or `=`, and the f-string around it.
    bare_field: Option<TextRange>,
    field_fstring: Option<ast::FStringFlags>,
    // The precedence the operator around the read asks of its operand, `None` where no operator asks for one.
    slot: Option<OperatorPrecedence>,
    // The read is in the head of a decorator, which before Python 3.9 must be a dotted name.
    decorator_head: bool,
    // The quotes of the f-strings that hold the read.
    fstring_quotes: Vec<Quote>,
    // The read is an argument of `all` or `any`, which stop at the first element that decides them.
    short_circuits: bool,
    // The read is the argument of `next` or `anext`, or the receiver of `send`, `throw`, `close` or `__next__`.
    consumes: bool,
    // The parameter whose default holds the read.
    default_parameter: Option<&'a ast::ParameterWithDefault>,
}

impl<'a> Reading<'a> {
    fn of(next: &'a Stmt, reference: TextRange) -> Self {
        let mut reading = Self {
            reference,
            statements: 0,
            repeats: 0,
            branches: 0,
            read: None,
            effects: false,
            whole: false,
            field_start: false,
            bare_field: None,
            field_fstring: None,
            slot: None,
            decorator_head: false,
            fstring_quotes: Vec::new(),
            short_circuits: false,
            consumes: false,
            default_parameter: None,
        };
        reading.visit_stmt(next);
        reading
    }

    fn in_branch(&mut self, visit: impl FnOnce(&mut Self)) {
        self.branches += 1;
        visit(self);
        self.branches -= 1;
    }
}

impl<'a> Visitor<'a> for Reading<'a> {
    fn visit_parameters(&mut self, parameters: &'a ast::Parameters) {
        if let Some(parameter) = parameters
            .iter_non_variadic_params()
            .find(|parameter| parameter.default().is_some_and(|default| default.range().contains_range(self.reference)))
        {
            self.default_parameter = Some(parameter);
        }
        visitor::walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.read.is_some() {
            return;
        }
        self.statements += 1;
        let whole = match stmt {
            Stmt::Return(ast::StmtReturn { value, .. }) => value.as_deref(),
            Stmt::Assign(assign) => Some(assign.value.as_ref()),
            _ => None,
        };
        self.whole |= whole.is_some_and(|value| value.range() == self.reference);
        match stmt {
            // The value is evaluated before the targets are stored.
            Stmt::Assign(assign) => {
                self.visit_expr(&assign.value);
                assign.targets.iter().for_each(|target| self.visit_expr(target));
            }
            // `python -O` skips an assertion, and a type alias evaluates its value lazily.
            Stmt::Assert(_) | Stmt::TypeAlias(_) => self.in_branch(|reading| visitor::walk_stmt(reading, stmt)),
            // Entering an item runs its `__enter__` before the later items are evaluated.
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                for item in items {
                    self.visit_with_item(item);
                    if self.read.is_none() {
                        self.effects = true;
                    }
                }
                self.visit_body(body);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
        self.statements -= 1;
    }

    // An `elif` or `else` clause runs only when the branches before it do not.
    fn visit_elif_else_clause(&mut self, clause: &'a ast::ElifElseClause) {
        self.statements += 1;
        visitor::walk_elif_else_clause(self, clause);
        self.statements -= 1;
    }

    // A `case` runs only when the cases before it do not match, and its guard and body run only when it matches.
    fn visit_match_case(&mut self, case: &'a ast::MatchCase) {
        self.in_branch(|reading| visitor::walk_match_case(reading, case));
    }

    // A handler's type is evaluated only when an exception reaches it.
    fn visit_except_handler(&mut self, handler: &'a ast::ExceptHandler) {
        self.in_branch(|reading| visitor::walk_except_handler(reading, handler));
    }

    // Annotations are not evaluated in a function body, and are evaluated lazily otherwise.
    fn visit_annotation(&mut self, expr: &'a Expr) {
        self.in_branch(|reading| visitor::walk_annotation(reading, expr));
    }

    // Type parameters are evaluated lazily.
    fn visit_type_params(&mut self, type_params: &'a ast::TypeParams) {
        self.in_branch(|reading| visitor::walk_type_params(reading, type_params));
    }

    fn visit_decorator(&mut self, decorator: &'a ast::Decorator) {
        if decorator.expression.range().contains_range(self.reference) {
            self.decorator_head = in_decorator_head(&decorator.expression, self.reference);
        }
        visitor::walk_decorator(self, decorator);
    }

    fn visit_interpolated_string_element(&mut self, element: &'a ast::InterpolatedStringElement) {
        if let ast::InterpolatedStringElement::Interpolation(interpolation) = element
            && interpolation.range.contains_range(self.reference)
        {
            self.field_start |= interpolation.expression.range().start() == self.reference.start();
            let bare = matches!(interpolation.conversion, ast::ConversionFlag::None)
                && interpolation.debug_text.is_none()
                && interpolation.format_spec.is_none();
            self.bare_field =
                (bare && interpolation.expression.range() == self.reference).then_some(interpolation.range);
        }
        visitor::walk_interpolated_string_element(self, element);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.read.is_some() {
            return;
        }
        if expr.range() == self.reference {
            self.read = Some(self.statements > 1 || self.repeats > 0 || self.branches > 0);
            return;
        }
        if expr.range().contains_range(self.reference) {
            self.slot = operand_precedence(expr, self.reference);
        }
        if let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr {
            let arguments = arguments.args.iter().chain(arguments.keywords.iter().map(|keyword| &keyword.value));
            let read_argument = arguments.into_iter().any(|argument| argument.range() == self.reference);
            self.whole |= read_argument;
            self.short_circuits |= read_argument && matches!(last_segment(func), Some("all" | "any"));
            let steps_argument = read_argument && matches!(last_segment(func), Some("next" | "anext"));
            let steps_receiver = matches!(
                func.as_ref(),
                Expr::Attribute(ast::ExprAttribute { value, attr, .. })
                    if value.range() == self.reference && matches!(attr.as_str(), "send" | "throw" | "close" | "__next__")
            );
            self.consumes |= steps_argument || steps_receiver;
        }
        if let Expr::FString(ast::ExprFString { value, .. }) = expr
            && expr.range().contains_range(self.reference)
        {
            self.fstring_quotes.extend(
                value
                    .f_strings()
                    .filter(|fstring| fstring.range.contains_range(self.reference))
                    .map(|fstring| fstring.flags.quote_style()),
            );
            self.field_fstring = value
                .f_strings()
                .find(|fstring| fstring.range.contains_range(self.reference))
                .map(|fstring| fstring.flags);
        }
        // An expression that does not hold the read finishes before it, so its calls and lookups run first.
        if !expr.range().contains_range(self.reference) && is_effect(expr) {
            self.effects = true;
            return;
        }
        let repeats = is_repeating_expr(expr);
        self.repeats += usize::from(repeats);
        match expr {
            // The test runs first, then only one of the two branches.
            Expr::If(ast::ExprIf { test, body, orelse, .. }) => {
                self.visit_expr(test);
                self.in_branch(|reading| reading.visit_expr(body));
                self.in_branch(|reading| reading.visit_expr(orelse));
            }
            // Each operand after the first runs only when the operands before it do not decide the result.
            Expr::BoolOp(ast::ExprBoolOp { values, .. }) => {
                let (first, rest) = values.split_at(values.len().min(1));
                for value in first {
                    self.visit_expr(value);
                }
                for value in rest {
                    self.in_branch(|reading| reading.visit_expr(value));
                }
            }
            // A chain stops at its first false link, so only the first comparator runs for sure.
            Expr::Compare(ast::ExprCompare { operands, .. }) => {
                let (first, rest) = operands.split_at(operands.len().min(2));
                for operand in first {
                    self.visit_expr(operand);
                }
                for operand in rest {
                    self.in_branch(|reading| reading.visit_expr(operand));
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
        self.repeats -= usize::from(repeats);
    }
}

// Before Python 3.9 a decorator takes a dotted name, optionally called, so only the callee of a call is in its head.
fn in_decorator_head(expr: &Expr, reference: TextRange) -> bool {
    match expr {
        Expr::Call(ast::ExprCall { func, arguments, .. }) => {
            !arguments.range().contains_range(reference) && in_decorator_head(func, reference)
        }
        _ => true,
    }
}

fn is_effect(expr: &Expr) -> bool {
    matches!(
        expr,
        Expr::Call(_)
            | Expr::Attribute(_)
            | Expr::Subscript(_)
            | Expr::Await(_)
            | Expr::Named(_)
            | Expr::Yield(_)
            | Expr::YieldFrom(_)
    )
}
