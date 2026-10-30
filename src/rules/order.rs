use std::collections::{HashMap, HashSet};
use std::ops::RangeInclusive;

use ruff_diagnostics::{Edit, Fix};
use ruff_python_ast::visitor::{self, Visitor};
use ruff_python_ast::{self as ast, Expr, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use crate::checker::Checker;
use crate::registry::Rule;
use crate::rules::{class_defs, statement_block};
use crate::semantic::{BindingKind, Evaluation};

pub fn definition_before_use(checker: &mut Checker) {
    let body = checker.body();
    reorder(checker, &Suite { body, separator: "\n\n" }, &module_uses(checker, body));
    for class in class_defs(body) {
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
            match reference.evaluation {
                Evaluation::Runtime => {
                    uses.runtime.insert(index);
                }
                Evaluation::Deferred if reference.statement != index => {
                    uses.deferred.entry(index).or_default().push((reference.statement, reference.range.start()));
                }
                Evaluation::Deferred | Evaluation::Annotation => {}
            }
        }
        if semantic.references.iter().any(|reference| {
            reference.statement == index
                && reference.evaluation == Evaluation::Runtime
                && semantic
                    .bindings
                    .get(reference.name)
                    .is_some_and(|bindings| bindings.iter().any(|binding| binding.kind != BindingKind::Import))
        }) {
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
    let mut runtime_names = RuntimeNames::default();
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
    for (index, stmt) in class.body.iter().enumerate() {
        let Some(name) = definition_name(stmt).map(ast::Identifier::as_str) else {
            continue;
        };
        if counts[name] > 1 || runtime_names.names.contains(name) {
            uses.runtime.insert(index);
            continue;
        }
        let members = &checker.semantic().member_references;
        for reference in members.iter().filter(|reference| reference.class == class.range && reference.name == name) {
            if let Some(user) =
                class.body.iter().position(|candidate| candidate.range().contains_range(reference.range))
                && user != index
            {
                uses.deferred.entry(index).or_default().push((user, reference.range.start()));
            }
        }
    }
    uses
}

#[derive(Default)]
struct RuntimeNames<'a> {
    names: HashSet<&'a str>,
}

impl<'a> Visitor<'a> for RuntimeNames<'a> {
    fn visit_expr(&mut self, expr: &'a Expr) {
        if let Expr::Name(name) = expr {
            self.names.insert(name.id.as_str());
        }
        visitor::walk_expr(self, expr);
    }
}

fn reorder(checker: &mut Checker, suite: &Suite, uses: &Uses) {
    let length = suite.body.len();
    let owners = owners(length, uses);
    let expected = expected_order(length, &owners, uses);
    let Some(low) = (0..length).find(|&position| expected[position] != position) else {
        return;
    };
    let high = (0..length).rfind(|&position| expected[position] != position).unwrap_or(low);
    let fix = reorder_fix(checker, suite, &expected, low..=high);
    for position in 1..length {
        let index = expected[position];
        let (Some(owner), Some(identifier)) = (owners[index], definition_name(&suite.body[index])) else {
            continue;
        };
        if index.checked_sub(1) == Some(expected[position - 1]) {
            continue;
        }
        let name = identifier.as_str();
        let owner_name = definition_name(&suite.body[owner]).map_or("the statement", ast::Identifier::as_str);
        let message = if index < owner {
            format!("`{name}` is defined before its first use in `{owner_name}`; move it below")
        } else {
            format!("`{name}` should directly follow `{owner_name}`, its first user")
        };
        checker.report(Rule::DefinitionBeforeUse, identifier.range(), message, fix.clone());
    }
}

fn owners(length: usize, uses: &Uses) -> Vec<Option<usize>> {
    let mut owners: Vec<Option<usize>> = (0..length)
        .map(|index| uses.deferred.get(&index).and_then(|users| users.iter().map(|(user, _)| *user).min()))
        .collect();
    let mut pinned: Vec<usize> = uses.runtime.iter().copied().collect();
    let mut seen: HashSet<usize> = pinned.iter().copied().collect();
    while let Some(index) = pinned.pop() {
        owners[index] = None;
        for (dependency, users) in &uses.deferred {
            if users.iter().any(|(user, _)| *user == index) && seen.insert(*dependency) {
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
        .comments()
        .any(|comment| region.contains_range(comment) && !moved.iter().any(|block| block.contains(comment.start())))
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

#[cfg(test)]
mod tests {
    use ruff_python_ast::PythonVersion;
    use test_case::test_case;

    use crate::registry::Rule;
    use crate::snapshot::check;

    #[test_case("SMP020.py" ; "definition_before_use")]
    #[test_case("SMP020_floating.py" ; "floating_comment")]
    #[test_case("SMP020_nonl.py" ; "without_trailing_newline")]
    fn definition_before_use(fixture: &str) {
        check(Rule::DefinitionBeforeUse, fixture, PythonVersion::PY314);
    }
}
