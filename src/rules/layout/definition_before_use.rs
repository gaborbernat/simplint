use std::mem;

use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast::visitor::{Visitor, walk_expr, walk_stmt};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::class_order::Arrangement;
use super::definition_time::{class_runtime_names, definition_name, definition_time_exprs};
use super::reorder::{Read, Suite, depth_first, moved_span, moves_definition_unsafely, reorder_fix};
use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::semantic::{BindingKind, Evaluation, Reference};
use crate::syntax::{signature_annotations, statement_index};

pub fn definition_before_use(checker: &Checker) {
    let body = checker.body();
    let runtime_names: FxHashMap<TextRange, FxHashSet<&str>> =
        checker.class_defs().iter().map(|class| (class.range, class_runtime_names(class))).collect();
    let uses = definition_uses(checker, body, &runtime_names);
    let mut owners = owners(body.len(), &uses);
    // A definition whose own statement reads names moves only where those names are still bound above it.
    let expected = loop {
        let expected = expected_order(body.len(), &owners, &uses);
        let mut position = vec![0; body.len()];
        for (at, &index) in expected.iter().enumerate() {
            position[index] = at;
        }
        let stuck: Vec<usize> = uses
            .header_reads
            .iter()
            .filter(|(reader, read)| {
                owners[**reader].is_some() && read.iter().any(|&bound| position[bound] > position[**reader])
            })
            .map(|(reader, _)| *reader)
            .collect();
        if stuck.is_empty() {
            break expected;
        }
        for reader in stuck {
            owners[reader] = None;
        }
    };
    report_misplaced(checker, &Suite { body, blank_lines: 2 }, &expected, &owners);
    let orders = checker.state().class_orders(checker);
    for class in checker.class_defs() {
        let order = &orders[&class.range];
        let expected = order.expected(Arrangement::Helpers);
        report_misplaced(checker, &Suite { body: &class.body, blank_lines: 1 }, &expected, &order.helper_owners());
    }
}

#[derive(Default)]
struct DefinitionUses {
    deferred: FxHashMap<usize, Vec<Read>>,
    // Read while the module loads, so they stay where they are.
    runtime: FxHashSet<usize>,
    // Run while the module loads, so what they call stays where it is too. A base class or a default value is read
    // there without running its body.
    invoked: FxHashSet<usize>,
    // The statements binding the names a definition's own statement reads, such as its defaults or decorators.
    header_reads: FxHashMap<usize, FxHashSet<usize>>,
}

fn definition_uses(
    checker: &Checker,
    body: &[Stmt],
    runtime_names: &FxHashMap<TextRange, FxHashSet<&str>>,
) -> DefinitionUses {
    let semantic = checker.semantic();
    let lazy = semantic.annotations_are_lazy();
    let mut eager = eager_annotations(checker);
    eager.sort_unstable_by_key(Ranged::start);
    let import_time = import_time_functions(body, runtime_names);
    // Some decorators read the annotations they decorate while the module loads, even when Python defers them. Without
    // lazy annotations, a `def` evaluates its own signature's annotations when it runs. A name a function only annotates
    // with is not a helper it calls.
    let evaluation = |reference: &Reference| match reference.evaluation {
        Evaluation::Annotation if in_ranges(&eager, reference.range) => {
            if reference.function.is_none() {
                Evaluation::Runtime
            } else {
                Evaluation::Deferred
            }
        }
        Evaluation::Annotation if !lazy && reference.function.is_none() => Evaluation::Runtime,
        Evaluation::Deferred if reference.function.is_some_and(|function| import_time.contains(&function)) => {
            Evaluation::Runtime
        }
        other => other,
    };
    // The statements that, as they run, read a name something other than an import above the first definition binds,
    // with the statements binding it. A definition moves past an import below it, so that import counts like any other
    // binding.
    let first_definition = body.iter().find(|stmt| definition_name(stmt).is_some()).map(Ranged::start);
    let mut runtime_readers: FxHashMap<usize, FxHashSet<usize>> = FxHashMap::default();
    for reference in semantic.references().filter(|reference| evaluation(reference) == Evaluation::Runtime) {
        let bound = semantic
            .bindings(reference.name)
            .iter()
            .filter(|binding| {
                binding.module_scope
                    && (binding.kind != BindingKind::Import
                        || first_definition.is_some_and(|start| binding.range.start() > start))
            })
            .map(|binding| statement_index(body, binding.range.start()))
            .filter(|&statement| statement != reference.statement);
        runtime_readers.entry(reference.statement).or_default().extend(bound);
    }
    let passive = passive_reads(body);
    let mut uses = DefinitionUses::default();
    for (index, stmt) in body.iter().enumerate() {
        let Some(name) = definition_name(stmt).map(ast::Identifier::as_str) else {
            continue;
        };
        if semantic.is_rebound(name) {
            uses.runtime.insert(index);
            continue;
        }
        for reference in semantic.references_to(name) {
            match evaluation(reference) {
                Evaluation::Runtime => {
                    uses.runtime.insert(index);
                    if !passive.contains(&reference.range) {
                        uses.invoked.insert(index);
                    }
                }
                Evaluation::Deferred if reference.statement != index => {
                    uses.deferred
                        .entry(index)
                        .or_default()
                        .push(Read { reader: reference.statement, at: reference.range.start() });
                }
                Evaluation::Deferred | Evaluation::Annotation => {}
            }
        }
        if let Some(bound) = runtime_readers.remove(&index).filter(|bound| !bound.is_empty()) {
            uses.header_reads.insert(index, bound);
        }
    }
    uses
}

// The names read while the module loads that only name a definition without running it: a base class, or a default
// value. A call anywhere above the name may run it.
fn passive_reads(body: &[Stmt]) -> FxHashSet<TextRange> {
    let mut finder = PassiveReads::default();
    finder.visit_body(body);
    finder.ranges
}

#[derive(Default)]
struct PassiveReads {
    ranges: FxHashSet<TextRange>,
}

impl PassiveReads {
    fn collect(&mut self, expr: &Expr) {
        match expr {
            Expr::Name(name) => {
                self.ranges.insert(name.range);
            }
            Expr::Attribute(attribute) => self.collect(&attribute.value),
            Expr::Subscript(subscript) => {
                self.collect(&subscript.value);
                self.collect(&subscript.slice);
            }
            Expr::Tuple(tuple) => tuple.elts.iter().for_each(|element| self.collect(element)),
            _ => {}
        }
    }
}

impl<'a> Visitor<'a> for PassiveReads {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::ClassDef(class) => {
                for base in class.bases() {
                    self.collect(base);
                }
                for keyword in class.keywords() {
                    self.collect(&keyword.value);
                }
            }
            Stmt::FunctionDef(function) => {
                for parameter in function.parameters.iter_non_variadic_params() {
                    if let Some(default) = parameter.default.as_deref() {
                        self.collect(default);
                    }
                }
            }
            _ => {}
        }
        walk_stmt(self, stmt);
    }
}

fn eager_annotations(checker: &Checker) -> Vec<TextRange> {
    let semantic = checker.semantic();
    let signatures = checker
        .function_defs()
        .iter()
        .filter(|function| semantic.reads_annotations(&function.decorator_list))
        .flat_map(|function| signature_annotations(function).map(Ranged::range));
    let fields =
        checker.class_defs().iter().filter(|class| semantic.reads_annotations(&class.decorator_list)).flat_map(
            |class| class.body.iter().filter_map(Stmt::as_ann_assign_stmt).map(|field| field.annotation.range()),
        );
    signatures.chain(fields).collect()
}

// The functions and lambdas that may run while the module loads, so what they read must already exist: a lambda in
// module-level code, as `(lambda: x)()` or a callback handed to a call, a function a class body calls, as in
// `tokens = [state(1)]`, and a `def` in a block of module-level code, such as an `if __name__ == "__main__":` body,
// which the block usually calls. A lambda bound straight to a name is a definition like any `def`.
fn import_time_functions(body: &[Stmt], runtime_names: &FxHashMap<TextRange, FxHashSet<&str>>) -> FxHashSet<TextRange> {
    let mut finder = ImportTimeFunctions { runtime_names, ranges: FxHashSet::default(), blocks: 0, in_class: false };
    finder.visit_body(body);
    finder.ranges
}

struct ImportTimeFunctions<'r> {
    runtime_names: &'r FxHashMap<TextRange, FxHashSet<&'r str>>,
    ranges: FxHashSet<TextRange>,
    blocks: usize,
    in_class: bool,
}

impl<'a> Visitor<'a> for ImportTimeFunctions<'_> {
    fn visit_stmt(&mut self, stmt: &'a Stmt) {
        match stmt {
            Stmt::FunctionDef(function) => {
                if self.blocks > 0 {
                    self.ranges.insert(function.range());
                }
                for expr in definition_time_exprs(function) {
                    self.visit_expr(expr);
                }
            }
            Stmt::ClassDef(class) => {
                let called = &self.runtime_names[&class.range];
                self.ranges.extend(
                    class
                        .body
                        .iter()
                        .filter_map(Stmt::as_function_def_stmt)
                        .filter(|function| called.contains(function.name.as_str()))
                        .map(Ranged::range),
                );
                let outer = (mem::take(&mut self.blocks), mem::replace(&mut self.in_class, true));
                walk_stmt(self, stmt);
                (self.blocks, self.in_class) = outer;
            }
            Stmt::If(_) | Stmt::Try(_) | Stmt::With(_) | Stmt::For(_) | Stmt::While(_) | Stmt::Match(_)
                if !self.in_class =>
            {
                self.blocks += 1;
                walk_stmt(self, stmt);
                self.blocks -= 1;
            }
            Stmt::Assign(ast::StmtAssign { targets, value, .. })
                if targets.len() == 1 && targets[0].is_name_expr() && value.is_lambda_expr() => {}
            _ => walk_stmt(self, stmt),
        }
    }

    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Lambda(lambda) = expr {
            self.ranges.insert(lambda.range);
        } else {
            walk_expr(self, expr);
        }
    }
}

// Whether a range lies in one of the annotations, sorted by start. Annotations never overlap, so the last one that
// starts at or before the range is the only one that can hold it.
fn in_ranges(ranges: &[TextRange], range: TextRange) -> bool {
    let next = ranges.partition_point(|candidate| candidate.start() <= range.start());
    next > 0 && ranges[next - 1].contains_range(range)
}

fn owners(length: usize, uses: &DefinitionUses) -> Vec<Option<usize>> {
    let mut owners: Vec<Option<usize>> = (0..length)
        .map(|index| uses.deferred.get(&index).and_then(|users| users.iter().map(|read| read.reader).min()))
        .collect();
    let mut dependencies: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    for (dependency, users) in &uses.deferred {
        for read in users {
            dependencies.entry(read.reader).or_default().push(*dependency);
        }
    }
    for &index in &uses.runtime {
        owners[index] = None;
    }
    let mut pinned: Vec<usize> = uses.invoked.iter().copied().collect();
    let mut seen: FxHashSet<usize> = pinned.iter().copied().collect();
    while let Some(index) = pinned.pop() {
        owners[index] = None;
        for dependency in dependencies.get(&index).into_iter().flatten() {
            if seen.insert(*dependency) {
                pinned.push(*dependency);
            }
        }
    }
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); length];
    for (index, owner) in owners.iter().enumerate() {
        if let Some(owner) = owner {
            children[*owner].push(index);
        }
    }
    // A chain that ends is never dropped, and nothing on it is either, so only the chains that loop are checked.
    let mut finite: Vec<bool> = owners.iter().map(Option::is_none).collect();
    let mut ends: Vec<usize> = (0..length).filter(|index| finite[*index]).collect();
    // An index has one owner, so the walk reaches each child once.
    while let Some(index) = ends.pop() {
        for &child in &children[index] {
            finite[child] = true;
        }
        ends.extend_from_slice(&children[index]);
    }
    // Walking each chain from its own index is quadratic in a long chain of helpers, so a dropped index marks the
    // indices whose chain runs into it instead, and an index is dropped when its owner is not marked.
    let mut marked = vec![false; length];
    for index in 0..length {
        let Some(owner) = owners[index].filter(|_| !finite[index]) else { continue };
        if marked[owner] {
            continue;
        }
        owners[index] = None;
        marked[index] = true;
        let mut upstream = vec![index];
        while let Some(node) = upstream.pop() {
            for &child in &children[node] {
                if !marked[child] {
                    marked[child] = true;
                    upstream.push(child);
                }
            }
        }
    }
    owners
}

fn expected_order(length: usize, owners: &[Option<usize>], uses: &DefinitionUses) -> Vec<usize> {
    let mut children: Vec<Vec<(TextSize, usize)>> = vec![Vec::new(); length];
    for (index, owner) in owners.iter().enumerate() {
        if let Some(owner) = owner {
            let first = uses.deferred[&index].iter().filter(|read| read.reader == *owner).map(|read| read.at).min();
            children[*owner].push((first.expect("the owner reads the helper"), index));
        }
    }
    let children: Vec<Vec<usize>> = children
        .into_iter()
        .map(|mut list| {
            list.sort_unstable();
            list.into_iter().map(|(_, child)| child).collect()
        })
        .collect();
    let mut order = Vec::with_capacity(length);
    depth_first((0..length).filter(|index| owners[*index].is_none()), &children, |index| order.push(index));
    order
}

// The first report carries the fix that moves every misplaced statement of the body.
fn report_misplaced(checker: &Checker, suite: &Suite, expected: &[usize], owners: &[Option<usize>]) {
    let Some(span) = moved_span(expected) else {
        return;
    };
    let mut reports = Vec::new();
    let describe = |index: usize| {
        definition_name(&suite.body[index]).map_or_else(
            || format!("the statement on line {}", checker.line_number(suite.body[index].start())),
            |name| format!("`{}`", name.as_str()),
        )
    };
    for position in 1..suite.body.len() {
        let index = expected[position];
        let (Some(owner), Some(identifier)) = (owners[index], definition_name(&suite.body[index])) else {
            continue;
        };
        let predecessor = expected[position - 1];
        if index.checked_sub(1) == Some(predecessor) {
            continue;
        }
        let name = identifier.as_str();
        let owner_name = describe(owner);
        let (message, title) = if index < owner {
            (
                format!("`{name}` is defined before its first use in {owner_name}; move it below {owner_name}"),
                format!("Move `{name}` below {owner_name}"),
            )
        } else if predecessor == owner {
            (
                format!("`{name}` should directly follow {owner_name}, its first user"),
                format!("Move `{name}` right after {owner_name}"),
            )
        } else {
            // The predecessor may be a nested helper `owner` never calls; `owner` is named only to contrast.
            let predecessor_name = describe(predecessor);
            (
                format!("`{name}` should directly follow {predecessor_name}, not {owner_name}, its first user"),
                format!("Move `{name}` right after {predecessor_name}"),
            )
        };
        reports.push((identifier.range(), message, title));
    }
    // One fix reorders the whole block, so only the block's first visible report carries it: Ruff renders a report's
    // fix in full, and repeating a block-sized edit on every report made output slow on files with many misplaced
    // definitions.
    checker.report_shared_fix(Rule::DefinitionBeforeUse, reports, || {
        reorder_fix(checker, suite, expected, span.clone(), shifts_unsafely)
    });
}

// A `def` that moves past a decorator or a default changes when Python evaluates it, so a definition whose move is
// unsafe makes the fix unsafe; the statements it moves past run in the order they had.
fn shifts_unsafely(stmt: &Stmt) -> bool {
    moves_definition_unsafely(stmt).unwrap_or(false)
}
