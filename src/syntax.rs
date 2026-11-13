// AST and source helpers that rules, the linter and the project index all read: walking statement bodies, what a
// name's underscores say, what a module's `__all__` and strings hold, and where the line above a statement starts.
use std::borrow::Cow;

use ruff_python_ast::AnyParameterRef;
use ruff_python_ast::helpers::{any_over_expr, is_dunder, map_callable, map_subscript};
use ruff_python_ast::name::UnqualifiedName;
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, OperatorPrecedence, Stmt};
use ruff_python_parser::parse_expression;
use ruff_python_stdlib::typing::{is_literal_member, is_pep_593_generic_member, is_standard_library_generic_member};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet};

// Ranges that nest or are disjoint, such as the scopes of a file, so that the ranges holding another range are found
// by a search instead of a scan.
#[derive(Default)]
pub struct Nesting {
    ranges: Vec<TextRange>,
    // The indices of `ranges` by where they start, outer ranges first, each with the position in this order of the
    // range it is nested in.
    order: Vec<(usize, Option<usize>)>,
}

impl Nesting {
    pub fn new(ranges: Vec<TextRange>) -> Self {
        let mut sorted: Vec<usize> = (0..ranges.len()).collect();
        sorted.sort_by(|&left, &right| {
            ranges[left].start().cmp(&ranges[right].start()).then(ranges[right].end().cmp(&ranges[left].end()))
        });
        let mut open: Vec<usize> = Vec::new();
        let mut order = Vec::with_capacity(sorted.len());
        for (position, &index) in sorted.iter().enumerate() {
            while open.last().is_some_and(|&top| ranges[sorted[top]].end() <= ranges[index].start()) {
                open.pop();
            }
            order.push((index, open.last().copied()));
            open.push(position);
        }
        Self { ranges, order }
    }

    pub fn range(&self, index: usize) -> TextRange {
        self.ranges[index]
    }

    // The indices of the ranges that hold `range`, the innermost first. A range that holds `range` starts no later
    // than it, and each such range encloses the last one that starts before it, so the holders are that one and the
    // ranges it is nested in. Exact for a non-empty `range`, and for an empty one that no two ranges touch at; there it
    // returns one of the two holders a linear scan finds.
    pub fn holders(&self, range: TextRange) -> impl Iterator<Item = usize> + '_ {
        let last = self.order.partition_point(|&(index, _)| self.ranges[index].start() <= range.start()).checked_sub(1);
        std::iter::successors(last, |&position| self.order[position].1)
            .map(|position| self.order[position].0)
            .filter(move |&index| self.ranges[index].contains_range(range))
    }
}

// A `\r\n` terminator is stepped over whole: one byte back is a `\n`, which `line_start` reads as a line of its own.
pub fn previous_line_start(source: &str, line_start: TextSize) -> TextSize {
    let terminator = if source[..line_start.to_usize()].ends_with("\r\n") { 2 } else { 1 };
    source.line_start(line_start - TextSize::from(terminator))
}

// What a name's leading underscores say about who may use it. The two readings of "public" differ on dunders:
// `from module import *` without `__all__` skips every name with a leading underscore, dunders included, while the
// typing spec and PEP 8 treat only a non-dunder underscore name as internal, so `__version__` is public.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Visibility {
    Public,
    Dunder,
    Private,
}

impl Visibility {
    pub(crate) fn of(name: &str) -> Self {
        if !name.starts_with('_') {
            Self::Public
        } else if is_dunder(name) {
            Self::Dunder
        } else {
            Self::Private
        }
    }
}

// The prefix of a private module named `_<prefix>_<rest>`, which a package would group with its siblings.
pub(crate) fn name_prefix(stem: &str) -> Option<&str> {
    let (prefix, rest) = stem.strip_prefix('_')?.split_once('_')?;
    (!prefix.is_empty() && !rest.is_empty()).then_some(prefix)
}

// Whether a function is an `@overload` stub, which the type checker reads in place of the implementation below it.
pub fn is_overload(decorators: &[ast::Decorator]) -> bool {
    has_decorator(decorators, &["overload"])
}

// Matches by last segment, so `typing.overload` and `overload` both match `"overload"`, and a called decorator by its
// factory, so `pytest.fixture(scope="module")` matches `"fixture"`.
pub fn has_decorator(decorators: &[ast::Decorator], names: &[&str]) -> bool {
    decorator_names(decorators).iter().any(|name| names.contains(name))
}

pub fn attribute_call<'a>(expr: &'a Expr, builtins: &[&str]) -> Option<(&'a Expr, &'a Expr)> {
    let ast::ExprCall { func, arguments, .. } = expr.as_call_expr()?;
    let [object, attribute, ..] = &*arguments.args else {
        return None;
    };
    builtins.contains(&func.as_name_expr()?.id.as_str()).then_some((object, attribute))
}

// The name an `import` binds in its scope: its `as` name, or the first part of a dotted module name, which `import a.b`
// binds as `a`.
pub fn alias_root_binding(alias: &ast::Alias) -> &str {
    alias.asname.as_ref().map_or_else(|| top_level(&alias.name), |name| name.as_str())
}

pub fn import_aliases(stmt: &Stmt) -> &[ast::Alias] {
    match stmt {
        Stmt::Import(ast::StmtImport { names, .. }) | Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => names,
        _ => &[],
    }
}

pub fn from_imports<'a>(body: &'a [Stmt], module: &'a str) -> impl Iterator<Item = &'a ast::StmtImportFrom> {
    body.iter().filter_map(move |stmt| match stmt {
        Stmt::ImportFrom(import)
            if import.level == 0 && import.module.as_ref().is_some_and(|name| name.as_str() == module) =>
        {
            Some(import)
        }
        _ => None,
    })
}

pub fn pattern_capture(pattern: &ast::Pattern) -> Option<&ast::Identifier> {
    match pattern {
        ast::Pattern::MatchAs(ast::PatternMatchAs { name, .. })
        | ast::Pattern::MatchStar(ast::PatternMatchStar { name, .. })
        | ast::Pattern::MatchMapping(ast::PatternMatchMapping { rest: name, .. }) => name.as_ref(),
        _ => None,
    }
}

pub fn sole_argument(call: &ast::ExprCall) -> Option<&Expr> {
    match call.arguments.args.as_ref() {
        [argument] if call.arguments.keywords.is_empty() => Some(argument),
        _ => None,
    }
}

pub fn argument<'a>(call: &'a ast::ExprCall, position: usize, keyword: &str) -> Option<&'a Expr> {
    call.arguments.args.get(position).or_else(|| {
        call.arguments
            .keywords
            .iter()
            .find(|item| item.arg.as_ref().is_some_and(|name| name.as_str() == keyword))
            .map(|item| &item.value)
    })
}

pub fn is_repeating_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::Lambda(_) | Expr::ListComp(_) | Expr::SetComp(_) | Expr::DictComp(_) | Expr::Generator(_))
}

pub fn keyword_is_true(expr: &Expr, arg: &str) -> bool {
    expr.as_call_expr()
        .and_then(|call| call.arguments.find_keyword(arg))
        .is_some_and(|keyword| is_true_literal(&keyword.value))
}

pub fn is_true_literal(expr: &Expr) -> bool {
    expr.as_boolean_literal_expr().is_some_and(|literal| literal.value)
}

pub fn dataclass_decorator<'a>(
    class: &'a ast::StmtClassDef,
    is_dataclass: &impl Fn(&Expr) -> bool,
) -> Option<&'a Expr> {
    class
        .decorator_list
        .iter()
        .find(|decorator| is_dataclass(decorator_callee(decorator)))
        .map(|decorator| &decorator.expression)
}

// What a dataclass's annotated class attribute declares: a field, or what its `ClassVar`, `InitVar` or `KW_ONLY`
// annotation makes it instead.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Declares {
    Field,
    ClassVar,
    InitVar,
    KwOnly,
}

impl Declares {
    pub fn of(annotation: &Expr) -> Self {
        match annotation_name(annotation) {
            Some("ClassVar") => Self::ClassVar,
            Some("InitVar") => Self::InitVar,
            Some("KW_ONLY") => Self::KwOnly,
            _ => Self::Field,
        }
    }
}

pub struct DataclassField<'a> {
    pub name: &'a str,
    pub stmt: &'a ast::StmtAnnAssign,
    pub kind: Declares,
}

// The class attributes a dataclass body annotates by name, in order.
pub fn dataclass_fields(body: &[Stmt]) -> impl Iterator<Item = DataclassField<'_>> {
    body.iter().filter_map(Stmt::as_ann_assign_stmt).filter_map(|stmt| {
        stmt.target.as_name_expr().map(|name| DataclassField {
            name: name.id.as_str(),
            stmt,
            kind: Declares::of(&stmt.annotation),
        })
    })
}

// The last segment of the name an annotation subscripts or names, read inside the quotes of a string annotation too.
pub fn annotation_name(annotation: &Expr) -> Option<&str> {
    match annotation.as_string_literal_expr() {
        Some(literal) => literal.value.to_str().split('[').next().map(|name| last_dotted_segment(name.trim())),
        None => last_segment(map_subscript(annotation)),
    }
}

// An attribute a method writes on its instance through its receiver, with the range that names it, the statement that
// writes it, the one after it in the same body, and whether the write is an `object.__setattr__` call, which a frozen
// dataclass makes.
pub struct InstanceWrite<'a> {
    pub attr: &'a str,
    pub range: TextRange,
    pub stmt: &'a Stmt,
    pub next: Option<&'a Stmt>,
    pub method: &'a ast::StmtFunctionDef,
    pub through_object: bool,
}

// Writes in nested defs count and nested classes do not. A class method's receiver is the class, so callers that
// want instances skip those.
pub fn instance_writes(class: &ast::StmtClassDef) -> Vec<InstanceWrite<'_>> {
    let mut writes = Vec::new();
    for method in module_statements(&class.body).filter_map(Stmt::as_function_def_stmt) {
        let Some(receiver) = receiver(method) else {
            continue;
        };
        for_each_body_outside_classes(&method.body, &mut |body| {
            for (index, stmt) in body.iter().enumerate() {
                let next = body.get(index + 1);
                if let Some(name) = set_through_object(stmt, receiver) {
                    let (attr, range) = (name.value.to_str(), name.range);
                    writes.push(InstanceWrite { attr, range, stmt, next, method, through_object: true });
                }
                for target in write_targets(stmt) {
                    if let Expr::Attribute(ast::ExprAttribute { value, attr, .. }) = target
                        && is_name(value, receiver)
                    {
                        let (range, attr) = (attr.range, attr.as_str());
                        writes.push(InstanceWrite { attr, range, stmt, next, method, through_object: false });
                    }
                }
            }
        });
    }
    writes
}

// The attributes a class's instance methods assign on their instance, each with its first assignment in the source.
pub fn instance_attributes(class: &ast::StmtClassDef) -> Vec<(&str, TextRange)> {
    let mut first: FxHashMap<&str, TextRange> = FxHashMap::default();
    for write in instance_writes(class) {
        if !write.through_object && !is_classmethod(write.method) && !matches!(write.stmt, Stmt::Delete(_)) {
            let range = first.entry(write.attr).or_insert(write.range);
            if write.range.start() < range.start() {
                *range = write.range;
            }
        }
    }
    let mut attributes: Vec<(&str, TextRange)> = first.into_iter().collect();
    attributes.sort_unstable_by_key(|(_, range)| range.start());
    attributes
}

// The statements that run when the module loads, including those of its top-level `if`, `try` and `match` blocks, where
// modules pick an implementation by version, availability or platform.
pub fn module_statements(body: &[Stmt]) -> impl Iterator<Item = &Stmt> {
    module_statements_with_next(body).map(|(stmt, _)| stmt)
}

// Each statement of `module_statements` with the one after it in its own block, where a docstring for it would sit.
pub fn module_statements_with_next(body: &[Stmt]) -> impl Iterator<Item = (&Stmt, Option<&Stmt>)> {
    body.iter().enumerate().flat_map(|(index, stmt)| -> Box<dyn Iterator<Item = (&Stmt, Option<&Stmt>)> + '_> {
        match stmt {
            Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => Box::new(
                module_statements_with_next(body)
                    .chain(elif_else_clauses.iter().flat_map(|clause| module_statements_with_next(&clause.body))),
            ),
            Stmt::Match(ast::StmtMatch { cases, .. }) => {
                Box::new(cases.iter().flat_map(|case| module_statements_with_next(&case.body)))
            }
            Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => Box::new(
                module_statements_with_next(body)
                    .chain(handlers.iter().flat_map(|handler| match handler {
                        ast::ExceptHandler::ExceptHandler(handler) => module_statements_with_next(&handler.body),
                    }))
                    .chain(module_statements_with_next(orelse))
                    .chain(module_statements_with_next(finalbody)),
            ),
            _ => Box::new(std::iter::once((stmt, body.get(index + 1)))),
        }
    })
}

pub fn receiver(method: &ast::StmtFunctionDef) -> Option<&str> {
    if has_decorator(&method.decorator_list, &["staticmethod"]) {
        return None;
    }
    first_positional(&method.parameters)
}

pub fn first_positional(parameters: &ast::Parameters) -> Option<&str> {
    parameters.posonlyargs.iter().chain(&parameters.args).next().map(|parameter| parameter.name().as_str())
}

// A class defined in a method has its own `self`, so its body is left out of the method's bodies.
fn for_each_body_outside_classes<'a>(body: &'a [Stmt], visit: &mut impl FnMut(&'a [Stmt])) {
    visit(body);
    for stmt in body.iter().filter(|stmt| !matches!(stmt, Stmt::ClassDef(_))) {
        for_each_child_body(stmt, &mut |child| for_each_body_outside_classes(child, visit));
    }
}

// The attribute an `object.__setattr__(receiver, "name", value)` call sets, which a frozen dataclass uses to set its
// own attributes.
fn set_through_object<'a>(stmt: &'a Stmt, receiver: &str) -> Option<&'a ast::ExprStringLiteral> {
    let call = stmt.as_expr_stmt()?.value.as_call_expr()?;
    let attribute = call.func.as_attribute_expr()?;
    let is_object = is_name(&attribute.value, "object");
    match call.arguments.args.as_ref() {
        [Expr::Name(target), Expr::StringLiteral(name), ..]
            if is_object && attribute.attr.as_str() == "__setattr__" && target.id.as_str() == receiver =>
        {
            Some(name)
        }
        _ => None,
    }
}

// The expressions a statement stores into, with tuples, lists and starred targets taken apart, in the order they are
// written.
fn write_targets(stmt: &Stmt) -> Vec<&Expr> {
    let mut pending: Vec<&Expr> = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, .. }) | Stmt::Delete(ast::StmtDelete { targets, .. }) => {
            targets.iter().collect()
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, .. })
        | Stmt::AugAssign(ast::StmtAugAssign { target, .. })
        | Stmt::For(ast::StmtFor { target, .. }) => vec![target.as_ref()],
        Stmt::With(ast::StmtWith { items, .. }) => {
            items.iter().filter_map(|item| item.optional_vars.as_deref()).collect()
        }
        _ => Vec::new(),
    };
    let mut targets = Vec::new();
    let mut next = 0;
    while let Some(&target) = pending.get(next) {
        next += 1;
        match target {
            Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
                pending.extend(elts);
            }
            Expr::Starred(ast::ExprStarred { value, .. }) => pending.push(value),
            _ => targets.push(target),
        }
    }
    targets
}

// Each expression `pick` maps to a value, in the order a `Visitor` walks them.
pub struct ExprCollector<F, T> {
    pick: F,
    pub found: Vec<T>,
}

impl<F, T> ExprCollector<F, T> {
    pub fn new<'a>(pick: F) -> Self
    where
        F: FnMut(&'a Expr) -> Option<T>,
    {
        Self { pick, found: Vec::new() }
    }
}

impl<'a, F: FnMut(&'a Expr) -> Option<T>, T> Visitor<'a> for ExprCollector<F, T> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Some(value) = (self.pick)(expr) {
            self.found.push(value);
        }
        visitor::walk_expr(self, expr);
    }
}

pub fn is_classmethod(method: &ast::StmtFunctionDef) -> bool {
    has_decorator(&method.decorator_list, &["classmethod"])
}

pub fn decorator_names(decorators: &[ast::Decorator]) -> Vec<&str> {
    decorators.iter().filter_map(|decorator| last_segment(decorator_callee(decorator))).collect()
}

pub fn decorator_callee(decorator: &ast::Decorator) -> &Expr {
    map_callable(&decorator.expression)
}

pub fn statement_index(body: &[Stmt], offset: TextSize) -> usize {
    body.partition_point(|stmt| stmt.end() <= offset)
}

pub enum Exports<'a> {
    Absent,
    Listed(Listing<'a>),
    Dynamic,
}

impl Exports<'_> {
    pub fn may_list(&self, name: &str) -> bool {
        match self {
            Self::Listed(listing) => listing.contains(name),
            Self::Dynamic => true,
            Self::Absent => false,
        }
    }

    pub fn lists(&self, name: &str) -> bool {
        matches!(self, Self::Listed(listing) if listing.contains(name))
    }
}

pub struct Listing<'a> {
    names: Vec<&'a str>,
    members: FxHashSet<&'a str>,
}

pub fn dunder_all(body: &[Stmt]) -> Exports<'_> {
    // An `__all__` bound inside an `if` or `try` depends on how the module runs, so its contents are unknown.
    let mut nested = false;
    for_each_body(body, &mut |statements, owner| {
        nested |= owner.is_some() && statements.iter().any(|stmt| all_binding(stmt).is_some());
    });
    if nested {
        return Exports::Dynamic;
    }
    // `from ._impl import __all__` re-exports a list this module cannot see.
    let imported = body.iter().any(|stmt| match stmt {
        Stmt::ImportFrom(ast::StmtImportFrom { names, .. }) => {
            names.iter().any(|alias| alias_binding(alias) == "__all__")
        }
        _ => false,
    });
    if imported {
        return Exports::Dynamic;
    }
    let mut exports = Exports::Absent;
    for stmt in body {
        let (resets, elements): (bool, Option<Vec<&Expr>>) = if let Some(binding) = all_binding(stmt) {
            (binding.operator.is_none(), all_literal(stmt).map(|literal| literal.elements.iter().collect()))
        } else if let Some((method, arguments)) = all_mutation(stmt) {
            let elements = match (method, &*arguments.args) {
                ("append", [element]) => Some(vec![element]),
                ("extend", [Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. })]) => {
                    Some(elts.iter().collect())
                }
                _ => None,
            };
            (false, elements)
        } else {
            continue;
        };
        let names: Option<Vec<&str>> = elements.and_then(|elements| {
            elements.iter().map(|element| Some(element.as_string_literal_expr()?.value.to_str())).collect()
        });
        exports = match (exports, names) {
            (_, None) | (Exports::Dynamic, _) => Exports::Dynamic,
            (Exports::Listed(mut listed), Some(names)) if !resets => {
                listed.extend(names);
                Exports::Listed(listed)
            }
            (_, Some(names)) => Exports::Listed(Listing::new(names)),
        };
    }
    exports
}

pub fn for_each_body<'a>(body: &'a [Stmt], visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    walk_bodies(body, None, visit);
}

fn walk_bodies<'a>(body: &'a [Stmt], owner: Option<&'a Stmt>, visit: &mut impl FnMut(&'a [Stmt], Option<&'a Stmt>)) {
    visit(body, owner);
    for stmt in body {
        for_each_child_body(stmt, &mut |child| walk_bodies(child, Some(stmt), visit));
    }
}

pub fn alias_binding(alias: &ast::Alias) -> &str {
    alias.asname.as_ref().unwrap_or(&alias.name).as_str()
}

pub fn all_literal(stmt: &Stmt) -> Option<AllLiteral<'_>> {
    // In `alias = __all__ = [...]` the list has a second name that can change it out of sight.
    if let Stmt::Assign(ast::StmtAssign { targets, .. }) = stmt
        && !targets.first().is_some_and(is_all_target)
    {
        return None;
    }
    match all_binding(stmt)? {
        AllBinding {
            value: Some(value @ (Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }))),
            operator: None | Some(ast::Operator::Add),
        } => Some(AllLiteral { value: value.range(), elements: elts }),
        _ => None,
    }
}

pub fn all_binding(stmt: &Stmt) -> Option<AllBinding<'_>> {
    match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.iter().any(is_all_target) => {
            Some(AllBinding { value: Some(value), operator: None })
        }
        Stmt::AnnAssign(ast::StmtAnnAssign { target, value, .. }) if is_all_target(target) => {
            Some(AllBinding { value: value.as_deref(), operator: None })
        }
        Stmt::AugAssign(ast::StmtAugAssign { target, op, value, .. }) if is_all_target(target) => {
            Some(AllBinding { value: Some(value), operator: Some(*op) })
        }
        _ => None,
    }
}

pub fn all_mutation(stmt: &Stmt) -> Option<(&str, &ast::Arguments)> {
    let call = stmt.as_expr_stmt()?.value.as_call_expr()?;
    let attribute = call.func.as_attribute_expr().filter(|attribute| is_all_target(&attribute.value))?;
    Some((attribute.attr.as_str(), &call.arguments))
}

// The statement binds `__all__` or changes it in place.
pub fn builds_all(stmt: &Stmt) -> bool {
    all_binding(stmt).is_some() || all_mutation(stmt).is_some()
}

impl<'a> Listing<'a> {
    fn new(names: Vec<&'a str>) -> Self {
        Self { members: names.iter().copied().collect(), names }
    }

    fn extend(&mut self, names: Vec<&'a str>) {
        self.members.extend(names.iter().copied());
        self.names.extend(names);
    }

    pub fn names(&self) -> &[&'a str] {
        &self.names
    }

    pub fn contains(&self, name: &str) -> bool {
        self.members.contains(name)
    }
}

// The names Ruff reads from the module's `__all__` bindings, in any block of the module but not in a function or class.
// Ruff reads the strings of a list, a tuple, a `list()` or `tuple()` of one, and the operands of a `+` chain, and no
// name that `extend` or `append` adds.
pub fn export_names(body: &[Stmt]) -> FxHashSet<&str> {
    let mut names = FxHashSet::default();
    for value in scope_statements(body).into_iter().filter_map(|stmt| all_binding(stmt)?.value) {
        add_export_names(value, &mut names);
    }
    names
}

// Every statement of the scope `body` opens, nested blocks included. Function and class bodies bind in their own scope,
// so they are skipped.
pub fn scope_statements(body: &[Stmt]) -> Vec<&Stmt> {
    fn collect<'a>(body: &'a [Stmt], found: &mut Vec<&'a Stmt>) {
        for stmt in body {
            found.push(stmt);
            if !matches!(stmt, Stmt::FunctionDef(_) | Stmt::ClassDef(_)) {
                for_each_child_body(stmt, &mut |child| collect(child, found));
            }
        }
    }
    let mut found = Vec::new();
    collect(body, &mut found);
    found
}

pub fn for_each_child_body<'a>(stmt: &'a Stmt, visit: &mut impl FnMut(&'a [Stmt])) {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { body, .. })
        | Stmt::ClassDef(ast::StmtClassDef { body, .. })
        | Stmt::With(ast::StmtWith { body, .. }) => visit(body),
        Stmt::For(ast::StmtFor { body, orelse, .. }) | Stmt::While(ast::StmtWhile { body, orelse, .. }) => {
            visit(body);
            visit(orelse);
        }
        Stmt::If(ast::StmtIf { body, elif_else_clauses, .. }) => {
            visit(body);
            for clause in elif_else_clauses {
                visit(&clause.body);
            }
        }
        Stmt::Try(ast::StmtTry { body, handlers, orelse, finalbody, .. }) => {
            visit(body);
            for ast::ExceptHandler::ExceptHandler(handler) in handlers {
                visit(&handler.body);
            }
            visit(orelse);
            visit(finalbody);
        }
        Stmt::Match(ast::StmtMatch { cases, .. }) => {
            for case in cases {
                visit(&case.body);
            }
        }
        _ => {}
    }
}

fn is_all_target(target: &Expr) -> bool {
    is_name(target, "__all__")
}

fn add_export_names<'a>(value: &'a Expr, names: &mut FxHashSet<&'a str>) {
    let Expr::BinOp(ast::ExprBinOp { left, right, .. }) = value else {
        return add_export_elements(value, names);
    };
    let mut left = left;
    let mut right = right;
    loop {
        add_export_elements(right, names);
        let Expr::BinOp(ast::ExprBinOp { left: inner_left, right: inner_right, .. }) = left.as_ref() else {
            return add_export_elements(left, names);
        };
        left = inner_left;
        right = inner_right;
    }
}

fn add_export_elements<'a>(expr: &'a Expr, names: &mut FxHashSet<&'a str>) {
    let elements = match expr {
        Expr::List(ast::ExprList { elts, .. }) | Expr::Tuple(ast::ExprTuple { elts, .. }) => elts,
        Expr::Call(ast::ExprCall { func, arguments, .. })
            if arguments.keywords.is_empty() && (is_name(func, "list") || is_name(func, "tuple")) =>
        {
            let [
                Expr::List(ast::ExprList { elts, .. })
                | Expr::Set(ast::ExprSet { elts, .. })
                | Expr::Tuple(ast::ExprTuple { elts, .. }),
            ] = &*arguments.args
            else {
                return;
            };
            elts
        }
        Expr::Named(ast::ExprNamed { value, .. }) => return add_export_elements(value, names),
        _ => return,
    };
    names.extend(elements.iter().filter_map(|element| Some(element.as_string_literal_expr()?.value.to_str())));
}

pub struct AllBinding<'a> {
    pub value: Option<&'a Expr>,
    // Set for `__all__ += ...`, which extends the list instead of replacing it.
    pub operator: Option<ast::Operator>,
}

pub struct AllLiteral<'a> {
    pub value: TextRange,
    pub elements: &'a [Expr],
}

pub fn target_names(target: &Expr) -> Vec<&str> {
    target_leaves(target).into_iter().filter_map(Expr::as_name_expr).map(|name| name.id.as_str()).collect()
}

pub fn target_leaves(target: &Expr) -> Vec<&Expr> {
    match target {
        Expr::Tuple(ast::ExprTuple { elts, .. }) | Expr::List(ast::ExprList { elts, .. }) => {
            elts.iter().flat_map(target_leaves).collect()
        }
        Expr::Starred(ast::ExprStarred { value, .. }) => target_leaves(value),
        _ => vec![target],
    }
}

// An f-string or t-string contributes each of its literal parts as a string of its own.
pub(crate) fn string_values(body: &[Stmt]) -> Vec<(TextRange, Cow<'_, str>)> {
    let mut values = StringValues { values: Vec::new(), with_prose: true };
    values.visit_body(body);
    values.values
}

// The identifier words of the strings Ruff parses as type expressions, the only strings that read a name: those in an
// annotation or type parameter, in an argument of a typing call that takes types, in a generic subscript, and the type
// of `Annotated`. A string anywhere else is data or prose.
pub(crate) fn type_string_words<'a>(body: &'a [Stmt], extend_generics: &'a [String]) -> FxHashSet<&'a str> {
    let mut strings = TypeStrings { words: FxHashSet::default(), in_type: false, extend_generics };
    strings.visit_body(body);
    strings.words
}

struct TypeStrings<'a> {
    words: FxHashSet<&'a str>,
    in_type: bool,
    extend_generics: &'a [String],
}

impl<'a> Visitor<'a> for TypeStrings<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { annotation, value, .. }) => {
                self.visit_in(annotation, true);
                if let Some(value) = value.as_deref().filter(|_| last_name(annotation) == Some("TypeAlias")) {
                    self.visit_in(value, true);
                }
            }
            Stmt::FunctionDef(function) => {
                for annotation in signature_annotations(function) {
                    self.visit_in(annotation, true);
                }
                self.visit_type_params(function.type_params.as_deref());
            }
            Stmt::ClassDef(class) => self.visit_type_params(class.type_params.as_deref()),
            Stmt::TypeAlias(alias) => {
                self.visit_in(&alias.value, true);
                self.visit_type_params(alias.type_params.as_deref());
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::StringLiteral(literal) => {
                if self.in_type {
                    self.words.extend(identifier_words(literal.value.to_str()));
                }
            }
            Expr::Call(ast::ExprCall { func, arguments, .. }) => {
                let takes_types = last_name(func).is_some_and(|name| {
                    matches!(
                        name,
                        "cast"
                            | "NamedTuple"
                            | "NewType"
                            | "ParamSpec"
                            | "TypeAliasType"
                            | "TypeForm"
                            | "TypedDict"
                            | "TypeVar"
                    )
                });
                let in_type = self.in_type || takes_types;
                self.visit_expr(func);
                for argument in &arguments.args {
                    self.visit_in(argument, in_type);
                }
                for keyword in &arguments.keywords {
                    self.visit_in(&keyword.value, in_type);
                }
            }
            Expr::Subscript(ast::ExprSubscript { value, slice, .. }) => {
                self.visit_expr(value);
                match last_name(value) {
                    Some(name) if is_literal_member(name) => self.visit_in(slice, false),
                    Some(name) if is_pep_593_generic_member(name) => match slice.as_ref() {
                        Expr::Tuple(ast::ExprTuple { elts, .. }) => {
                            for (index, element) in elts.iter().enumerate() {
                                self.visit_in(element, index == 0);
                            }
                        }
                        _ => self.visit_expr(slice),
                    },
                    Some(name) if is_standard_library_generic_member(name) || self.is_extended_generic(name) => {
                        self.visit_in(slice, true);
                    }
                    _ => self.visit_expr(slice),
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

fn last_name(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::Name(name) => Some(name.id.as_str()),
        Expr::Attribute(attribute) => Some(attribute.attr.as_str()),
        _ => None,
    }
}

// Splits at each character that is not a letter, digit or underscore, so an identifier stays one piece. A separator at
// either end, or two in a row, yields an empty piece, which a caller that needs only words skips.
pub fn identifier_words(text: &str) -> impl Iterator<Item = &str> {
    text.split(|char: char| !(char.is_alphanumeric() || char == '_'))
}

impl<'a> TypeStrings<'a> {
    fn visit_type_params(&mut self, type_params: Option<&'a ast::TypeParams>) {
        for expr in type_param_exprs(type_params) {
            self.visit_in(expr, true);
        }
    }

    fn visit_in(&mut self, expr: &'a Expr, in_type: bool) {
        let outer = std::mem::replace(&mut self.in_type, in_type);
        self.visit_expr(expr);
        self.in_type = outer;
    }

    // Ruff matches the qualified name of a subscripted value against `extend-generics`; this walk has no resolution,
    // so it matches the last name the value is written with, as it does for the typing generics.
    fn is_extended_generic(&self, name: &str) -> bool {
        self.extend_generics.iter().any(|target| last_dotted_segment(target) == name)
    }
}

// The strings a name may be spelled in to patch or look it up, which leaves out the prose of the statements that hold a
// string alone, such as docstrings.
pub(crate) fn code_string_values(body: &[Stmt]) -> Vec<(TextRange, Cow<'_, str>)> {
    let mut values = StringValues { values: Vec::new(), with_prose: false };
    values.visit_body(body);
    values.values
}

struct StringValues<'a> {
    values: Vec<(TextRange, Cow<'a, str>)>,
    with_prose: bool,
}

impl<'a> Visitor<'a> for StringValues<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        if self.with_prose || !is_prose_statement(stmt) {
            visitor::walk_stmt(self, stmt);
        }
    }

    fn visit_string_literal(&mut self, string_literal: &'a ast::StringLiteral) {
        self.values.push((string_literal.range, Cow::Borrowed(&string_literal.value)));
    }

    fn visit_bytes_literal(&mut self, bytes_literal: &'a ast::BytesLiteral) {
        self.values.push((bytes_literal.range, String::from_utf8_lossy(&bytes_literal.value)));
    }

    fn visit_interpolated_string_element(&mut self, element: &'a ast::InterpolatedStringElement) {
        if let ast::InterpolatedStringElement::Literal(literal) = element {
            self.values.push((literal.range, Cow::Borrowed(&literal.value)));
        }
        visitor::walk_interpolated_string_element(self, element);
    }
}

// A string a statement holds alone is prose, unless it is a doctest, whose examples read the names of the module.
pub(crate) fn is_prose_statement(stmt: &Stmt) -> bool {
    matches!(stmt, Stmt::Expr(ast::StmtExpr { value, .. })
        if value.as_string_literal_expr().is_some_and(|literal| !literal.value.to_str().contains(">>>")))
}

// Ruff's own return visitor stopped reporting generators, so a function's `return` statements and whether it yields
// are collected here, skipping nested scopes.
#[derive(Default)]
pub struct ReturnVisitor<'a> {
    pub returns: Vec<&'a ast::StmtReturn>,
    pub is_generator: bool,
}

impl<'a> Visitor<'a> for ReturnVisitor<'a> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(_) | Stmt::ClassDef(_) => {}
            Stmt::Return(ret) => {
                self.returns.push(ret);
                visitor::walk_stmt(self, stmt);
            }
            _ => visitor::walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Yield(_) | Expr::YieldFrom(_) => self.is_generator = true,
            // A lambda that yields is a generator of its own, so the function around it is not one.
            Expr::Lambda(_) => {}
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// Whether an expression is the condition of one of a comprehension's `if` clauses.
pub fn is_comprehension_condition(generators: &[ast::Comprehension], expr: &Expr) -> bool {
    generators.iter().any(|generator| generator.ifs.iter().any(|condition| std::ptr::eq(condition, expr)))
}

pub fn is_main_guard(stmt: &Stmt) -> bool {
    stmt.as_if_stmt().is_some_and(|stmt| is_main_test(&stmt.test))
}

pub fn is_main_test(test: &Expr) -> bool {
    let Expr::Compare(ast::ExprCompare { ops, operands, .. }) = test else {
        return false;
    };
    let is_main =
        |expr: &Expr| expr.as_string_literal_expr().is_some_and(|literal| literal.value.to_str() == "__main__");
    match (&**ops, &**operands) {
        ([ast::CmpOp::Eq], [left, right]) => {
            (is_name(left, "__name__") && is_main(right)) || (is_main(left) && is_name(right, "__name__"))
        }
        _ => false,
    }
}

// The methods that tell `pickle` and `copy` how to rebuild an object, in place of calling its class again.
pub(crate) const REBUILD_HOOKS: [&str; 4] = ["__reduce__", "__reduce_ex__", "__getnewargs__", "__getnewargs_ex__"];

// The callables that build a mock, which the testing rules and the receiver tracking of the project share.
pub(crate) const MOCK_CLASSES: &[&str] =
    &["Mock", "MagicMock", "AsyncMock", "NonCallableMock", "NonCallableMagicMock", "PropertyMock", "create_autospec"];

// The names the annotations of a module spell, including those of string annotations, which Ruff reads as uses, and
// those of type aliases and type parameters, which Ruff reads once the module binds the names they spell.
pub(crate) fn annotated_names(body: &[Stmt]) -> FxHashSet<String> {
    let mut names = AnnotatedNames::default();
    names.visit_body(body);
    names.0
}

#[derive(Default)]
struct AnnotatedNames(FxHashSet<String>);

impl<'a> Visitor<'a> for AnnotatedNames {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::AnnAssign(ast::StmtAnnAssign { annotation, .. }) => self.0.extend(annotation_names(annotation)),
            Stmt::FunctionDef(function) => {
                self.0.extend(signature_annotations(function).flat_map(annotation_names));
                self.0.extend(type_param_exprs(function.type_params.as_deref()).flat_map(annotation_names));
            }
            Stmt::ClassDef(class) => {
                self.0.extend(type_param_exprs(class.type_params.as_deref()).flat_map(annotation_names));
            }
            Stmt::TypeAlias(alias) => {
                self.0.extend(annotation_names(&alias.value));
                self.0.extend(type_param_exprs(alias.type_params.as_deref()).flat_map(annotation_names));
            }
            _ => {}
        }
        visitor::walk_stmt(self, stmt);
    }
}

// Leaves out the strings of an annotation, which the module's type strings read.
fn annotation_names(expr: &Expr) -> Vec<String> {
    let mut names = Vec::new();
    TypeNames { names: &mut names, strings: false }.visit_expr(expr);
    names
}

pub fn signature_annotations(function: &ast::StmtFunctionDef) -> impl Iterator<Item = &Expr> {
    function.parameters.iter().filter_map(AnyParameterRef::annotation).chain(function.returns.as_deref())
}

fn type_param_exprs(type_params: Option<&ast::TypeParams>) -> impl Iterator<Item = &Expr> {
    type_params.into_iter().flat_map(|params| &params.type_params).flat_map(|param| {
        let bound = match param {
            ast::TypeParam::TypeVar(ast::TypeParamTypeVar { bound, .. }) => bound.as_deref(),
            ast::TypeParam::TypeVarTuple(_) | ast::TypeParam::ParamSpec(_) => None,
        };
        bound.into_iter().chain(param.default())
    })
}

pub(crate) fn names_in(expr: &Expr) -> Vec<String> {
    let mut names = Vec::new();
    TypeNames { names: &mut names, strings: true }.visit_expr(expr);
    names
}

struct TypeNames<'n> {
    names: &'n mut Vec<String>,
    strings: bool,
}

impl<'a> Visitor<'a> for TypeNames<'_> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        match expr {
            Expr::Name(name) => self.names.push(name.id.to_string()),
            Expr::StringLiteral(literal) if self.strings => {
                if let Ok(parsed) = parse_expression(literal.value.to_str()) {
                    TypeNames { names: self.names, strings: true }.visit_expr(parsed.expr());
                }
            }
            _ => visitor::walk_expr(self, expr),
        }
    }
}

// Whether an `if` test is `TYPE_CHECKING`, bare or as `typing.TYPE_CHECKING`, which is false at run time. A module that
// binds it to a true constant runs the block, so the name is an ordinary flag there.
pub fn is_type_checking_name(test: &Expr) -> bool {
    match test {
        Expr::Name(name) => name.id.as_str() == "TYPE_CHECKING",
        Expr::Attribute(attribute) => attribute.attr.as_str() == "TYPE_CHECKING",
        _ => false,
    }
}

pub(crate) fn is_stdlib_root(minor: u8, module: &str) -> bool {
    ruff_python_stdlib::sys::is_known_standard_library(minor, top_level(module))
}

pub(crate) fn top_level(module: &str) -> &str {
    module.split_once('.').map_or(module, |(root, _)| root)
}

pub fn is_lazy(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::Import(ast::StmtImport { is_lazy: true, .. })
            | Stmt::ImportFrom(ast::StmtImportFrom { is_lazy: true, .. })
    )
}

pub fn docstring(body: &[Stmt]) -> Option<&ast::ExprStringLiteral> {
    match body.first()? {
        Stmt::Expr(ast::StmtExpr { value, .. }) => value.as_string_literal_expr(),
        _ => None,
    }
}

pub fn needs_parentheses(value: &Expr) -> bool {
    OperatorPrecedence::from_expr(value) < OperatorPrecedence::CallAttribute
        || matches!(value, Expr::Tuple(tuple) if !tuple.parenthesized)
}

pub fn expr_mentions(expr: &Expr, name: &str) -> bool {
    any_over_expr(expr, |expr| is_name(expr, name))
}

pub fn is_name(expr: &Expr, name: &str) -> bool {
    expr.as_name_expr().is_some_and(|found| found.id.as_str() == name)
}

// A module-level `if sys.version_info < ...: raise ...` or `sys.exit(...)` exists to run on interpreters older than
// the target and report them, so fixes must not add syntax or imports those interpreters lack.
pub fn has_version_gate(body: &[Stmt]) -> bool {
    body.iter().filter_map(Stmt::as_if_stmt).any(|gate| {
        any_over_expr(&gate.test, &|expr: &Expr| last_segment(expr) == Some("version_info"))
            && gate.body.last().is_some_and(|last| match last {
                Stmt::Raise(_) => true,
                Stmt::Expr(ast::StmtExpr { value, .. }) => {
                    value.as_call_expr().is_some_and(|call| last_segment(&call.func) == Some("exit"))
                }
                _ => false,
            })
    })
}

pub fn last_segment(expr: &Expr) -> Option<&str> {
    UnqualifiedName::from_expr(expr)?.segments().last().copied()
}

pub fn last_dotted_segment(dotted: &str) -> &str {
    dotted.rsplit_once('.').map_or(dotted, |(_, last)| last)
}

pub fn single_assignment(stmt: &Stmt) -> Option<(&ast::ExprName, Option<&Expr>, &Expr)> {
    let (target, annotation, value) = match stmt {
        Stmt::Assign(ast::StmtAssign { targets, value, .. }) if targets.len() == 1 => (&targets[0], None, value),
        Stmt::AnnAssign(ast::StmtAnnAssign { target, annotation, value: Some(value), .. }) => {
            (target.as_ref(), Some(annotation.as_ref()), value)
        }
        _ => return None,
    };
    Some((target.as_name_expr()?, annotation, value))
}

// Whether a function with this body is a generator: a `yield` outside any nested function, class or lambda.
pub fn yields(body: &[Stmt]) -> bool {
    let mut visitor = ReturnVisitor::default();
    visitor.visit_body(body);
    visitor.is_generator
}
