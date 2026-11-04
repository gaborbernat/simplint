use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::helpers::is_dunder;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, ExprContext, Pattern, Stmt};
use ruff_text_size::{Ranged, TextRange};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::layout::joined_width;
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::logic::inline::{declared_outer, fits_line, for_each_scoped_body, inlined_text, leading_name};
use crate::rules::{needs_parentheses, statement_deletion};
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
    // A whole argument, return or assigned value needs no parentheses, though a tuple still does.
    let parenthesize = needs_parentheses(value) && (!reading.whole || value.is_tuple_expr());
    let replacement = inlined_text(checker, value, value_range, parenthesize);
    let wrapping = replacement.chars().count() - value_text.chars().count();
    if !fits(checker, next, use_range, value_range, wrapping, depth) {
        return;
    }
    let deletion = Edit::range_deletion(deletion);
    let replace = Edit::range_replacement(replacement, use_range);
    let fix =
        if reading.effects { Fix::unsafe_edits(deletion, [replace]) } else { Fix::safe_edits(deletion, [replace]) };
    checker.report_fix(
        Rule::SingleUseVariable,
        assign.range,
        format!("`{name}` is used once, right below; inline its value"),
        Some(fix),
        format!("Inline `{name}`"),
    );
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
struct Reading {
    reference: TextRange,
    // The statements the walk is in, the next statement counting as one.
    statements: usize,
    // The lambdas and comprehensions the walk is in.
    repeats: usize,
    // Whether the read is conditional, set once the walk reaches it.
    read: Option<bool>,
    effects: bool,
    // The read is a whole argument, keyword value, return value or assigned value, where no parentheses are needed.
    whole: bool,
}

impl Reading {
    fn of(next: &Stmt, reference: TextRange) -> Self {
        let mut reading = Self { reference, statements: 0, repeats: 0, read: None, effects: false, whole: false };
        reading.visit_stmt(next);
        reading
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

    fn visit_expr(&mut self, expr: &'a Expr) {
        if self.read.is_some() {
            return;
        }
        if expr.range() == self.reference {
            self.read = Some(self.statements > 1 || self.repeats > 0);
            return;
        }
        if let Expr::Call(ast::ExprCall { arguments, .. }) = expr {
            let arguments = arguments.args.iter().chain(arguments.keywords.iter().map(|keyword| &keyword.value));
            self.whole |= arguments.into_iter().any(|argument| argument.range() == self.reference);
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
                self.visit_expr(body);
                self.visit_expr(orelse);
            }
            _ => visitor::walk_expr(self, expr),
        }
        self.repeats -= usize::from(repeats);
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
