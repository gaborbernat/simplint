use ruff_python_ast::helpers::any_over_expr;
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, OperatorPrecedence, Stmt, StringFlags};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::FxHashSet;

use crate::syntax::{is_name, is_repeating_expr, last_segment};

#[expect(clippy::struct_excessive_bools, reason = "each flag marks one place the read sits in")]
pub(super) struct ReadSite<'a> {
    // Whether the read is conditional, set once the walk reaches it. A read the walk never reaches stays unset.
    pub(super) read: Option<bool>,
    // Something before the read may have side effects, so inlining would reorder them.
    effects: bool,
    // The read is a whole argument, keyword value, return value or assigned value, where no parentheses are needed.
    pub(super) whole: bool,
    // The read starts its f-string field, so a value that starts with `{` would follow the field's `{`.
    pub(super) field_start: bool,
    // The read sits in the format spec of a field, where `{` and `}` are not escapes.
    pub(super) in_format_spec: bool,
    // The read is in a t-string, whose fields keep their source text.
    pub(super) in_tstring: bool,
    // The read is a part of its value, an attribute or a subscript that is not the callee of a call.
    reads_part: bool,
    // The whole field the read fills, `{name}`, with no conversion, format spec or `=`, and the f-string around it.
    pub(super) bare_field: Option<TextRange>,
    pub(super) field_fstring: Option<ast::FStringFlags>,
    // The precedence the operator around the read asks of its operand, `None` where no operator asks for one.
    pub(super) slot: Option<OperatorPrecedence>,
    // The read is in the head of a decorator, which before Python 3.9 must be a dotted name.
    pub(super) decorator_head: bool,
    // The quotes of the f-strings that hold the read.
    pub(super) fstring_quotes: Vec<Quote>,
    // The read is an argument of `all` or `any`, which stop at the first element that decides them.
    pub(super) short_circuits: bool,
    // The read is the argument of `next` or `anext`, or the receiver of `send`, `throw`, `close` or `__next__`.
    pub(super) consumes: bool,
    // The parameter whose default holds the read.
    pub(super) default_parameter: Option<&'a ast::ParameterWithDefault>,
    // The names read before the read, other than the callee of a call, whose binding a call in the value could change.
    names_before: Vec<&'a str>,
    // A name read before the read is formatted or unpacked there, which takes the state of its object at that point.
    state_before: bool,
}

impl<'a> ReadSite<'a> {
    pub(super) fn of(next: &'a Stmt, reference: TextRange) -> Self {
        let mut walk = ReadWalk {
            reference,
            statements: 0,
            repeats: 0,
            branches: 0,
            takes_state: 0,
            callee: None,
            site: Self {
                read: None,
                effects: false,
                whole: false,
                field_start: false,
                in_format_spec: false,
                in_tstring: false,
                reads_part: false,
                bare_field: None,
                field_fstring: None,
                slot: None,
                decorator_head: false,
                fstring_quotes: Vec::new(),
                short_circuits: false,
                consumes: false,
                default_parameter: None,
                names_before: Vec::new(),
                state_before: false,
            },
        };
        walk.visit_stmt(next);
        walk.site
    }
}

struct ReadWalk<'a> {
    reference: TextRange,
    // The statements the walk is in, the next statement counting as one.
    statements: usize,
    // The lambdas and comprehensions the walk is in.
    repeats: usize,
    // The branches the walk is in, where a part of the code may be skipped or evaluated later.
    branches: usize,
    // How many f-string fields, `*` and `**` unpackings the walk is in, outside the read.
    takes_state: usize,
    // The callee of the call the walk is in, which names the function or method rather than reading state.
    callee: Option<TextRange>,
    site: ReadSite<'a>,
}

impl<'a> ReadWalk<'a> {
    fn in_branch(&mut self, visit: impl FnOnce(&mut Self)) {
        self.branches += 1;
        visit(self);
        self.branches -= 1;
    }

    // Visits an expression whose value is formatted or unpacked as soon as it is evaluated.
    fn taking_state(&mut self, expr: &'a Expr) {
        let outside = !expr.range().contains_range(self.reference);
        self.takes_state += usize::from(outside);
        self.visit_expr(expr);
        self.takes_state -= usize::from(outside);
    }

    // Reading a part of the value lets its object die right after the read, and a t-string keeps its fields' source
    // text.
    fn places_read(&mut self, expr: &Expr) {
        match expr {
            Expr::Attribute(ast::ExprAttribute { value, .. })
                if value.range() == self.reference && self.callee != Some(expr.range()) =>
            {
                self.site.reads_part = true;
            }
            Expr::Subscript(ast::ExprSubscript { value, .. }) if value.range() == self.reference => {
                self.site.reads_part = true;
            }
            Expr::TString(_) if expr.range().contains_range(self.reference) => self.site.in_tstring = true,
            _ => {}
        }
    }
}

impl<'a> Visitor<'a> for ReadWalk<'a> {
    fn visit_parameters(&mut self, parameters: &'a ast::Parameters) {
        if let Some(parameter) = parameters
            .iter_non_variadic_params()
            .find(|parameter| parameter.default().is_some_and(|default| default.range().contains_range(self.reference)))
        {
            self.site.default_parameter = Some(parameter);
        }
        visitor::walk_parameters(self, parameters);
    }

    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.site.read.is_some() {
            return;
        }
        self.statements += 1;
        let whole = match stmt {
            Stmt::Return(ast::StmtReturn { value, .. }) => value.as_deref(),
            Stmt::Assign(assign) => Some(assign.value.as_ref()),
            _ => None,
        };
        self.site.whole |= whole.is_some_and(|value| value.range() == self.reference);
        match stmt {
            // The value is evaluated before the targets are stored.
            Stmt::Assign(assign) => {
                self.visit_expr(&assign.value);
                assign.targets.iter().for_each(|target| self.visit_expr(target));
            }
            // The target is loaded before the value, and an attribute or subscript target looks itself up first.
            Stmt::AugAssign(ast::StmtAugAssign { target, value, .. }) => {
                if !target.range().contains_range(self.reference) {
                    match target.as_ref() {
                        Expr::Name(name) => {
                            self.site.state_before = true;
                            self.site.names_before.push(name.id.as_str());
                        }
                        _ => self.site.effects = true,
                    }
                }
                self.visit_expr(value);
                self.visit_expr(target);
            }
            // `python -O` skips an assertion, and a type alias evaluates its value lazily.
            Stmt::Assert(_) | Stmt::TypeAlias(_) => self.in_branch(|reading| visitor::walk_stmt(reading, stmt)),
            // Entering an item runs its `__enter__` before the later items are evaluated.
            Stmt::With(ast::StmtWith { items, body, .. }) => {
                for item in items {
                    self.visit_with_item(item);
                    if self.site.read.is_none() {
                        self.site.effects = true;
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
            self.site.decorator_head = in_decorator_head(&decorator.expression, self.reference);
        }
        visitor::walk_decorator(self, decorator);
    }

    fn visit_keyword(&mut self, keyword: &'a ast::Keyword) {
        if keyword.arg.is_none() {
            self.taking_state(&keyword.value);
        } else {
            visitor::walk_keyword(self, keyword);
        }
    }

    fn visit_interpolated_string_element(&mut self, element: &'a ast::InterpolatedStringElement) {
        let ast::InterpolatedStringElement::Interpolation(interpolation) = element else {
            return;
        };
        let formats = !interpolation.range.contains_range(self.reference);
        if !formats {
            self.site.in_format_spec |=
                interpolation.format_spec.as_ref().is_some_and(|spec| spec.range.contains_range(self.reference));
            self.site.field_start |= interpolation.expression.range().start() == self.reference.start();
            let bare = matches!(interpolation.conversion, ast::ConversionFlag::None)
                && interpolation.debug_text.is_none()
                && interpolation.format_spec.is_none();
            self.site.bare_field =
                (bare && interpolation.expression.range() == self.reference).then_some(interpolation.range);
        }
        self.takes_state += usize::from(formats);
        visitor::walk_interpolated_string_element(self, element);
        self.takes_state -= usize::from(formats);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.site.read.is_some() {
            return;
        }
        if expr.range() == self.reference {
            self.site.read = Some(self.statements > 1 || self.repeats > 0 || self.branches > 0);
            return;
        }
        if expr.range().contains_range(self.reference) {
            self.site.slot = operand_precedence(expr, self.reference);
        }
        self.places_read(expr);
        if let Expr::Name(name) = expr
            && name.ctx.is_load()
            && self.callee != Some(name.range)
        {
            self.site.state_before |= self.takes_state > 0;
            self.site.names_before.push(name.id.as_str());
        }
        if let Expr::Call(ast::ExprCall { func, arguments, .. }) = expr {
            self.callee = Some(func.range());
            let arguments = arguments.args.iter().chain(arguments.keywords.iter().map(|keyword| &keyword.value));
            let read_argument = arguments.into_iter().any(|argument| argument.range() == self.reference);
            self.site.whole |= read_argument;
            self.site.short_circuits |= read_argument && matches!(last_segment(func), Some("all" | "any"));
            let steps_argument = read_argument && matches!(last_segment(func), Some("next" | "anext"));
            let steps_receiver = matches!(
                func.as_ref(),
                Expr::Attribute(ast::ExprAttribute { value, attr, .. })
                    if value.range() == self.reference
                        && matches!(attr.as_str(), "send" | "throw" | "close" | "__next__")
            );
            self.site.consumes |= steps_argument || steps_receiver;
        }
        if let Expr::FString(ast::ExprFString { value, .. }) = expr
            && expr.range().contains_range(self.reference)
        {
            self.site.fstring_quotes.extend(
                value
                    .f_strings()
                    .filter(|fstring| fstring.range.contains_range(self.reference))
                    .map(|fstring| fstring.flags.quote_style()),
            );
            self.site.field_fstring = value
                .f_strings()
                .find(|fstring| fstring.range.contains_range(self.reference))
                .map(|fstring| fstring.flags);
        }
        // An expression that does not hold the read finishes before it, so its calls and lookups run first.
        if !expr.range().contains_range(self.reference) && is_effect(expr) {
            self.site.effects = true;
            return;
        }
        // An operator that does not hold the read computes its result before the read, so the names it reads are taken.
        let operator = !expr.range().contains_range(self.reference)
            && matches!(expr, Expr::BinOp(_) | Expr::UnaryOp(_) | Expr::Compare(_) | Expr::BoolOp(_));
        self.takes_state += usize::from(operator);
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
            Expr::Starred(ast::ExprStarred { value, .. }) => self.taking_state(value),
            Expr::Dict(ast::ExprDict { items, .. }) => {
                for item in items {
                    match &item.key {
                        Some(key) => {
                            self.visit_expr(key);
                            self.visit_expr(&item.value);
                        }
                        None => self.taking_state(&item.value),
                    }
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
        self.takes_state -= usize::from(operator);
        self.repeats -= usize::from(repeats);
    }
}

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

// Whether inlining moves the value's evaluation past code that observes what the value changes: a call, lookup or
// await before the read, or, when the value itself calls or looks something up, a name read before the read that is
// formatted or unpacked there, or that a call can rebind. The value's object also dies with the read of one of its
// parts, which a call can finalize, and a walrus in the value can rebind a name read before the read.
pub(super) fn reorders(rebindable: &FxHashSet<&str>, assign: &ast::StmtAssign, reading: &ReadSite) -> bool {
    reading.effects
        || (any_over_expr(&assign.value, &is_effect)
            && (reading.state_before || reading.names_before.iter().any(|name| rebindable.contains(name))))
        || (reading.reads_part && any_over_expr(&assign.value, &Expr::is_call_expr))
        || any_over_expr(&assign.value, &|expr: &Expr| {
            matches!(expr, Expr::Named(ast::ExprNamed { target, .. })
                if reading.names_before.iter().any(|name| is_name(target, name)))
        })
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

pub(super) fn is_effect(expr: &Expr) -> bool {
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
