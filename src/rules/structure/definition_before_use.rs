use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, Stmt};
use ruff_source_file::LineRanges;
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::lint::checker::Checker;
use crate::lint::registry::Rule;
use crate::rules::ExprCollector;
use crate::semantic::{BindingKind, Evaluation, Reference};

pub fn definition_before_use(checker: &Checker) {
    let body = checker.body();
    reorder(checker, &Suite { body, separator: "\n\n" }, &module_uses(checker, body));
    for class in checker.class_defs() {
        reorder(checker, &Suite { body: &class.body, separator: "\n" }, &class_uses(checker, class));
    }
}

struct Suite<'a> {
    body: &'a [Stmt],
    separator: &'static str,
}

#[derive(Default)]
struct Uses {
    deferred: HashMap<usize, Vec<(usize, TextSize)>>,
    runtime: HashSet<usize>,
}

fn module_uses(checker: &Checker, body: &[Stmt]) -> Uses {
    let semantic = checker.semantic();
    let lazy = semantic.annotations_are_lazy();
    // Without lazy annotations, a `def` evaluates its own signature's annotations when it runs; annotations inside a
    // function body wait until that function is called.
    let evaluation = |reference: &Reference| match reference.evaluation {
        Evaluation::Annotation if !lazy && reference.function.is_none() => Evaluation::Runtime,
        Evaluation::Annotation if !lazy => Evaluation::Deferred,
        other => other,
    };
    // The statements that, as they run, read a name something other than an import binds.
    let runtime_readers: HashSet<usize> = semantic
        .references()
        .filter(|reference| {
            evaluation(reference) == Evaluation::Runtime
                && semantic.bindings(reference.name).iter().any(|binding| binding.kind != BindingKind::Import)
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
                    uses.deferred.entry(index).or_default().push((reference.statement, reference.range.start()));
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

fn definition_name(stmt: &Stmt) -> Option<&ast::Identifier> {
    match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { name, .. }) | Stmt::ClassDef(ast::StmtClassDef { name, .. }) => {
            Some(name)
        }
        _ => None,
    }
}

fn class_uses(checker: &Checker, class: &ast::StmtClassDef) -> Uses {
    let mut uses = Uses::default();
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for name in class.body.iter().filter_map(definition_name) {
        *counts.entry(name.as_str()).or_default() += 1;
    }
    let mut runtime_names = ExprCollector::new(|expr| expr.as_name_expr().map(|name| name.id.as_str()));
    for stmt in &class.body {
        match stmt {
            Stmt::FunctionDef(function) => {
                for decorator in &function.decorator_list {
                    runtime_names.visit_expr(&decorator.expression);
                }
                for default in
                    function.parameters.iter_non_variadic_params().filter_map(|parameter| parameter.default.as_deref())
                {
                    runtime_names.visit_expr(default);
                }
            }
            other => runtime_names.visit_stmt(other),
        }
    }
    let runtime_names: HashSet<&str> = runtime_names.found.into_iter().collect();
    for (index, stmt) in class.body.iter().enumerate() {
        let Some(name) = definition_name(stmt).map(ast::Identifier::as_str) else {
            continue;
        };
        if counts[name] > 1 || runtime_names.contains(name) {
            uses.runtime.insert(index);
            continue;
        }
        for reference in checker.semantic().member_references(class.range, name) {
            let reader = class.body.partition_point(|candidate| candidate.end() <= reference.start());
            if class.body.get(reader).is_some_and(|candidate| candidate.range().contains_range(*reference))
                && reader != index
            {
                uses.deferred.entry(index).or_default().push((reader, reference.start()));
            }
        }
    }
    uses
}

fn reorder(checker: &Checker, suite: &Suite, uses: &Uses) {
    let length = suite.body.len();
    let owners = owners(length, uses);
    let expected = expected_order(length, &owners, uses);
    let Some(low) = (0..length).find(|&position| expected[position] != position) else {
        return;
    };
    let high = (0..length).rfind(|&position| expected[position] != position).unwrap_or(low);
    // One fix reorders the whole block, so only the block's first report carries it: Ruff renders a report's fix in full,
    // and repeating a block-sized edit on every report made output slow on files with many misplaced definitions.
    let mut offered = false;
    for position in 1..length {
        let index = expected[position];
        let (Some(owner), Some(identifier)) = (owners[index], definition_name(&suite.body[index])) else {
            continue;
        };
        if index.checked_sub(1) == Some(expected[position - 1]) {
            continue;
        }
        let name = identifier.as_str();
        let owner_name = definition_name(&suite.body[owner]).map_or_else(
            || {
                let line = checker.source[..suite.body[owner].start().to_usize()].matches('\n').count() + 1;
                format!("the statement on line {line}")
            },
            |owner| format!("`{}`", owner.as_str()),
        );
        let (message, title) = if index < owner {
            (
                format!("`{name}` is defined before its first use in {owner_name}; move it below {owner_name}"),
                format!("Move `{name}` below {owner_name}"),
            )
        } else {
            (
                format!("`{name}` should directly follow {owner_name}, its first user"),
                format!("Move `{name}` right after {owner_name}"),
            )
        };
        let fix = if offered { None } else { reorder_fix(checker, suite, &expected, low..=high) };
        offered = true;
        checker.report_fix(Rule::DefinitionBeforeUse, identifier.range(), message, fix, title);
    }
}

fn owners(length: usize, uses: &Uses) -> Vec<Option<usize>> {
    let mut owners: Vec<Option<usize>> = (0..length)
        .map(|index| uses.deferred.get(&index).and_then(|users| users.iter().map(|(user, _)| *user).min()))
        .collect();
    let mut dependencies: HashMap<usize, Vec<usize>> = HashMap::new();
    for (dependency, users) in &uses.deferred {
        for (user, _) in users {
            dependencies.entry(*user).or_default().push(*dependency);
        }
    }
    let mut pinned: Vec<usize> = uses.runtime.iter().copied().collect();
    let mut seen: HashSet<usize> = pinned.iter().copied().collect();
    while let Some(index) = pinned.pop() {
        owners[index] = None;
        for dependency in dependencies.get(&index).into_iter().flatten() {
            if seen.insert(*dependency) {
                pinned.push(*dependency);
            }
        }
    }
    for index in 0..length {
        let mut current = owners[index];
        let mut steps = 0;
        while let Some(owner) = current {
            if owner == index || steps > length {
                owners[index] = None;
                break;
            }
            current = owners[owner];
            steps += 1;
        }
    }
    owners
}

fn expected_order(length: usize, owners: &[Option<usize>], uses: &Uses) -> Vec<usize> {
    let mut children: HashMap<usize, Vec<(TextSize, usize)>> = HashMap::new();
    for (index, owner) in owners.iter().enumerate() {
        if let Some(owner) = owner {
            let first = uses.deferred[&index].iter().filter(|(user, _)| user == owner).map(|(_, offset)| *offset).min();
            children.entry(*owner).or_default().push((first.unwrap_or_default(), index));
        }
    }
    for list in children.values_mut() {
        list.sort_unstable();
    }
    let mut order = Vec::with_capacity(length);
    let mut stack: Vec<usize> = (0..length).filter(|index| owners[*index].is_none()).rev().collect();
    while let Some(index) = stack.pop() {
        order.push(index);
        if let Some(list) = children.get(&index) {
            stack.extend(list.iter().rev().map(|(_, child)| *child));
        }
    }
    order
}

fn reorder_fix(checker: &Checker, suite: &Suite, expected: &[usize], span: RangeInclusive<usize>) -> Option<Fix> {
    let source = checker.source;
    let blocks: Vec<TextRange> = suite.body.iter().map(|stmt| statement_block(source, stmt)).collect();
    let moved = &blocks[span.clone()];
    let region = TextRange::new(moved[0].start(), moved[moved.len() - 1].end());
    if checker
        .comment_ranges()
        .comments_in_range(region)
        .iter()
        .any(|comment| !moved.iter().any(|block| block.contains(comment.start())))
        || moved.windows(2).any(|pair| pair[0].end() > pair[1].start())
    {
        return None;
    }
    let text: Vec<String> = expected[span.clone()]
        .iter()
        .map(|index| {
            let block = &source[blocks[*index]];
            if block.ends_with('\n') { block.to_owned() } else { format!("{block}\n") }
        })
        .collect();
    let mut replacement = text.join(suite.separator);
    if !source[region].ends_with('\n') {
        replacement.pop();
    }
    let moved_unsafe = span.filter(|position| expected[*position] != *position).any(|position| {
        matches!(&suite.body[expected[position]], Stmt::ClassDef(_))
            || matches!(&suite.body[expected[position]], Stmt::FunctionDef(function) if !function.decorator_list.is_empty())
    });
    let edit = Edit::range_replacement(replacement, region);
    Some(if moved_unsafe { Fix::unsafe_edit(edit) } else { Fix::safe_edit(edit) })
}

fn statement_block(source: &str, stmt: &Stmt) -> TextRange {
    let start = match stmt {
        Stmt::FunctionDef(ast::StmtFunctionDef { decorator_list, .. })
        | Stmt::ClassDef(ast::StmtClassDef { decorator_list, .. }) => {
            decorator_list.first().map_or(stmt.start(), Ranged::start)
        }
        _ => stmt.start(),
    };
    let mut line_start = source.line_start(start);
    // A byte order mark is not a line to step back into.
    while line_start > source.bom_start_offset() {
        let previous = source.line_start(line_start - TextSize::from(1));
        if !source[TextRange::new(previous, line_start)].trim_start().starts_with('#') {
            break;
        }
        line_start = previous;
    }
    TextRange::new(line_start, source.full_line_end(stmt.end()))
}
