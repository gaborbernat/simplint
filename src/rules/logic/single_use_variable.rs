use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::str::Quote;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, PythonVersion, Stmt, StringFlags};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::layout::joined_width;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::inline::{
    declared_outer, fits_line, for_each_scoped_body, inlined_text, leading_name, requoted_text,
};
use crate::rules::{keep_debug_text, needs_parentheses, statement_deletion};
use crate::syntax::{for_each_child_body, is_name, is_repeating_expr};

// Built-ins that read the locals by name, so a variable they read cannot be inlined away.
const LOCALS_READERS: [&str; 4] = ["eval", "exec", "locals", "vars"];

pub fn single_use_variable(checker: &Checker) {
    let names: FxHashMap<TextRange, Names> =
        checker.function_defs().iter().map(|function| (function.range, Names::of(&function.body))).collect();
    for_each_scoped_body(checker.body(), None, 0, &enter_function, &mut |body, function, depth| {
        let Some(function) = function else {
            return;
        };
        let names = &names[&function.range];
        for pair in body.windows(2) {
            if let [Stmt::Assign(assign), next] = pair {
                check_pair(checker, names, assign, next, depth);
            }
        }
    });
}

// `function` is the innermost `def` around a body; only statements inside a function are checked.
fn enter_function<'a>(function: Option<&'a ast::StmtFunctionDef>, stmt: &'a Stmt) -> Option<&'a ast::StmtFunctionDef> {
    match stmt {
        Stmt::FunctionDef(definition) => Some(definition),
        Stmt::ClassDef(_) => None,
        _ => function,
    }
}

fn check_pair<'a>(checker: &Checker<'a>, names: &Names<'a>, assign: &'a ast::StmtAssign, next: &'a Stmt, depth: usize) {
    let [Expr::Name(target)] = assign.targets.as_slice() else {
        return;
    };
    let name = target.id.as_str();
    let value = assign.value.as_ref();
    // A `while` test runs again on every pass, and `_` is a placeholder rather than a name to read.
    if name == "_"
        || is_dunder(name)
        || matches!(value, Expr::Yield(_) | Expr::YieldFrom(_) | Expr::Await(_) | Expr::Named(_))
        || matches!(next, Stmt::While(_))
        || checker.has_comment_in(TextRange::new(assign.start(), next.start()))
        || names.declared_outer.contains(name)
        || returns_or_yields(next, name)
        || leads_if_test(next, name)
        || raises_with_name(next, name)
        || names.rebound.contains(name)
        || names.reads_locals
    {
        return;
    }
    let others: Vec<(TextRange, ExprContext)> =
        names.mentions.get(name).into_iter().flatten().copied().filter(|(range, _)| *range != target.range).collect();
    let [(use_range, ExprContext::Load)] = others.as_slice() else {
        return;
    };
    let use_range = *use_range;
    if !next.range().contains_range(use_range) {
        return;
    }
    let reading = Reading::of(next, use_range);
    if reading.read != Some(false) {
        return;
    }
    let Some(deletion) = statement_deletion(checker, assign.range, None, false) else {
        return;
    };
    let value_range = checker.parenthesized_range(value, assign.into());
    let value_text = checker.text(value_range);
    // The width is measured on one line, so a value the source spreads over lines is left alone.
    if value_text.contains(['\n', '\r']) {
        return;
    }
    // A replacement field cannot hold a backslash before Python 3.12 (PEP 701).
    if checker.settings.from_ruff.target_version < PythonVersion::PY312
        && !reading.fstring_quotes.is_empty()
        && value_text.contains('\\')
    {
        return;
    }
    // Before Python 3.9 a decorator is a dotted name, optionally called, so the value must be one.
    if checker.settings.from_ruff.target_version < PythonVersion::PY39
        && reading.decorator_head
        && !is_dotted_name(value)
    {
        return;
    }
    // A whole argument, return or assigned value needs no parentheses, though a tuple still does.
    let parenthesize = needs_parentheses(value) && (!reading.whole || value.is_tuple_expr());
    let Some(mut replacement) = field_value(checker, value, value_range, parenthesize, &reading.fstring_quotes) else {
        return;
    };
    // A value that starts with `{` would make the field's `{` an escaped `{{`, so the value gets a space.
    if reading.field_start
        && replacement.starts_with('{')
        && checker.source[..use_range.start().to_usize()].ends_with('{')
    {
        replacement = format!(" {replacement} ");
    }
    let deletion = Edit::range_deletion(deletion);
    let edits = keep_debug_text(
        checker.source,
        std::slice::from_ref(next),
        vec![Edit::range_replacement(replacement, use_range)],
    );
    let fix = match edits {
        Some(edits) => {
            // A read in a `=` debug field rewrites the whole field, so the width is measured on the field it becomes.
            let Some(replace) = edits.first() else { return };
            let wrapping = replace.content().unwrap_or_default().chars().count() - value_text.chars().count();
            if !fits(checker, next, replace.range(), value_range, wrapping, depth) {
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

// The value as it goes into the statement. Before Python 3.12 a replacement field cannot hold the quote of the f-string
// around it, so a value that holds it takes the other quote in its strings, which needs every such f-string to share one
// quote.
fn field_value(
    checker: &Checker,
    value: &Expr,
    value_range: TextRange,
    parenthesize: bool,
    quotes: &[Quote],
) -> Option<String> {
    let before_312 = checker.settings.from_ruff.target_version < PythonVersion::PY312;
    let value_text = checker.text(value_range);
    if !(before_312 && quotes.iter().any(|quote| value_text.contains(quote.as_char()))) {
        return Some(inlined_text(checker, value, value_range, parenthesize));
    }
    let quote = quotes.first().filter(|first| quotes.iter().all(|quote| quote == *first))?;
    let text = requoted_text(checker, value, *quote)?;
    Some(if parenthesize { format!("({text})") } else { text })
}

// `return name` and `yield name` are left to Ruff's RET504, which covers the return case.
fn returns_or_yields(next: &Stmt, name: &str) -> bool {
    let value = match next {
        Stmt::Return(ast::StmtReturn { value, .. }) => value.as_deref(),
        Stmt::Expr(ast::StmtExpr { value, .. }) => match value.as_ref() {
            Expr::Yield(ast::ExprYield { value, .. }) => value.as_deref(),
            _ => None,
        },
        _ => None,
    };
    value.is_some_and(|value| is_name(value, name))
}

// Ruff's EM101 and EM102 ask for the message of an exception to be a variable, so the `raise` keeps it.
fn raises_with_name(next: &Stmt, name: &str) -> bool {
    let Stmt::Raise(ast::StmtRaise { exc: Some(exc), .. }) = next else {
        return false;
    };
    exc.as_call_expr().is_some_and(|call| call.arguments.args.iter().any(|argument| is_name(argument, name)))
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

// The formatter splits a statement wider than its line width over several lines. The width is judged on the
// statement's header, which ends where its first block starts, joined on one line at its block's indentation, with the
// value written in place of the name.
fn fits(
    checker: &Checker,
    next: &Stmt,
    use_range: TextRange,
    value_range: TextRange,
    wrapping: usize,
    depth: usize,
) -> bool {
    let (source, tokens) = (checker.source, checker.parsed.tokens());
    let mut header_end = next.end();
    for_each_child_body(next, &mut |block| {
        if let Some(first) = block.first() {
            header_end = header_end.min(first.start());
        }
    });
    let header = TextRange::new(next.start(), header_end);
    let (Some(header), Some(value)) = (joined_width(source, tokens, header), joined_width(source, tokens, value_range))
    else {
        return false;
    };
    let name = checker.text(use_range).chars().count();
    fits_line(checker, depth, header, name, value, wrapping)
}

// Every mention of each name in a function, found in one walk of its body, so a function with many assignments is
// walked once rather than once per assignment.
struct Names<'a> {
    mentions: FxHashMap<&'a str, Vec<(TextRange, ExprContext)>>,
    // Names the function binds in a way the walk does not follow: a `def`, a `class`, an import, an `except` target or
    // a pattern capture.
    rebound: FxHashSet<&'a str>,
    declared_outer: FxHashSet<&'a str>,
    // Whether the function reads its locals by name, which keeps every name alive.
    reads_locals: bool,
}

impl<'a> Names<'a> {
    fn of(body: &'a [Stmt]) -> Self {
        let mut names = Self {
            mentions: FxHashMap::default(),
            rebound: FxHashSet::default(),
            declared_outer: declared_outer(body),
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

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.mentions.entry(name.id.as_str()).or_default().push((name.range, name.ctx));
            self.reads_locals |= LOCALS_READERS.contains(&name.id.as_str());
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

// How the next statement reads the name, walked in evaluation order. `read` is set once the walk reaches the read, to
// whether it runs only in some passes or branches: in a nested block, a lambda or a comprehension. A read the walk
// never reaches stays unset, and is not reported. `effects` is set when something before the read may have side
// effects, so inlining would reorder them.
#[expect(clippy::struct_excessive_bools, reason = "each flag marks one place the read sits in")]
struct Reading {
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
    // The read is in the head of a decorator, which before Python 3.9 must be a dotted name.
    decorator_head: bool,
    // The quotes of the f-strings that hold the read.
    fstring_quotes: Vec<Quote>,
}

impl Reading {
    fn of(next: &Stmt, reference: TextRange) -> Self {
        let mut reading = Self {
            reference,
            statements: 0,
            repeats: 0,
            branches: 0,
            read: None,
            effects: false,
            whole: false,
            field_start: false,
            decorator_head: false,
            fstring_quotes: Vec::new(),
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

impl<'a> Visitor<'a> for Reading {
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
        if let Expr::Call(ast::ExprCall { arguments, .. }) = expr {
            let arguments = arguments.args.iter().chain(arguments.keywords.iter().map(|keyword| &keyword.value));
            self.whole |= arguments.into_iter().any(|argument| argument.range() == self.reference);
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
