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
use crate::syntax::signature_annotations;

pub fn definition_before_use(checker: &Checker) {
    let body = checker.body();
    let runtime_names: FxHashMap<TextRange, FxHashSet<&str>> =
        checker.class_defs().iter().map(|class| (class.range, class_runtime_names(class))).collect();
    let uses = module_uses(checker, body, &runtime_names);
    let owners = owners(body.len(), &uses);
    let expected = expected_order(body.len(), &owners, &uses);
    report_misplaced(checker, &Suite { body, blank_lines: 2 }, &expected, &owners);
    let orders = checker.state().class_orders(checker);
    for class in checker.class_defs() {
        let order = &orders[&class.range];
        let expected = order.expected(Arrangement::Helpers);
        report_misplaced(checker, &Suite { body: &class.body, blank_lines: 1 }, &expected, &order.helper_owners());
    }
}

#[derive(Default)]
struct Uses {
    deferred: FxHashMap<usize, Vec<Read>>,
    runtime: FxHashSet<usize>,
}

fn module_uses(checker: &Checker, body: &[Stmt], runtime_names: &FxHashMap<TextRange, FxHashSet<&str>>) -> Uses {
    let semantic = checker.semantic();
    let lazy = semantic.annotations_are_lazy();
    let mut eager = eager_annotations(checker);
    eager.sort_unstable_by_key(Ranged::start);
    let import_time = import_time_functions(body, runtime_names);
    // Decorators such as FastAPI routes, pydantic's `validate_call` or `dataclass` read the annotations they decorate
    // while the module loads, even when Python defers them. Without lazy annotations, a `def` evaluates its own
    // signature's annotations when it runs. A name a function only annotates with is not a helper it calls.
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
    // The statements that, as they run, read a name something other than an import above the first definition binds. A
    // definition moves past an import below it, so that import pins its readers like any other binding.
    let first_definition = body.iter().find(|stmt| definition_name(stmt).is_some()).map(Ranged::start);
    let runtime_readers: FxHashSet<usize> = semantic
        .references()
        .filter(|reference| {
            evaluation(reference) == Evaluation::Runtime
                && semantic.bindings(reference.name).iter().any(|binding| {
                    binding.kind != BindingKind::Import
                        || first_definition.is_some_and(|start| binding.range.start() > start)
                })
        })
        .map(|reference| reference.statement)
        .collect();
    let mut uses = Uses::default();
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
        if runtime_readers.contains(&index) {
            uses.runtime.insert(index);
        }
    }
    uses
}

// The annotations a decorator sees: the signature of a decorated function and the class-level annotations of a
// decorated class.
fn eager_annotations(checker: &Checker) -> Vec<TextRange> {
    let signatures = checker
        .function_defs()
        .iter()
        .filter(|function| !function.decorator_list.is_empty())
        .flat_map(|function| signature_annotations(function).map(Ranged::range));
    let fields =
        checker.class_defs().iter().filter(|class| !class.decorator_list.is_empty()).flat_map(|class| {
            class.body.iter().filter_map(Stmt::as_ann_assign_stmt).map(|field| field.annotation.range())
        });
    signatures.chain(fields).collect()
}

// Whether a range lies in one of the annotations, sorted by start. Annotations never overlap, so the last one that
// starts at or before the range is the only one that can hold it.
fn in_ranges(ranges: &[TextRange], range: TextRange) -> bool {
    let next = ranges.partition_point(|candidate| candidate.start() <= range.start());
    next > 0 && ranges[next - 1].contains_range(range)
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

// Reports each helper that sits away from its place in `expected`. The first report carries the fix that moves every
// misplaced statement of the body.
fn report_misplaced(checker: &Checker, suite: &Suite, expected: &[usize], owners: &[Option<usize>]) {
    let length = suite.body.len();
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
    for position in 1..length {
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

fn owners(length: usize, uses: &Uses) -> Vec<Option<usize>> {
    let mut owners: Vec<Option<usize>> = (0..length)
        .map(|index| uses.deferred.get(&index).and_then(|users| users.iter().map(|read| read.reader).min()))
        .collect();
    let mut dependencies: FxHashMap<usize, Vec<usize>> = FxHashMap::default();
    for (dependency, users) in &uses.deferred {
        for read in users {
            dependencies.entry(read.reader).or_default().push(*dependency);
        }
    }
    let mut pinned: Vec<usize> = uses.runtime.iter().copied().collect();
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
    while let Some(index) = ends.pop() {
        for &child in &children[index] {
            if !finite[child] {
                finite[child] = true;
                ends.push(child);
            }
        }
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

fn expected_order(length: usize, owners: &[Option<usize>], uses: &Uses) -> Vec<usize> {
    let mut children: Vec<Vec<(TextSize, usize)>> = vec![Vec::new(); length];
    for (index, owner) in owners.iter().enumerate() {
        if let Some(owner) = owner {
            let first = uses.deferred[&index].iter().filter(|read| read.reader == *owner).map(|read| read.at).min();
            children[*owner].push((first.unwrap_or_default(), index));
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
