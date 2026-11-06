use rustc_hash::{FxHashMap, FxHashSet};

use ruff_python_ast::visitor::Visitor;
use ruff_python_ast::{self as ast, ExprContext, Stmt};
use ruff_text_size::{Ranged, TextRange, TextSize};

use super::constructors::alternative_constructors;
use super::definition_time::{bound_names, definition_name, definition_time_exprs, signature_annotations};
use crate::lint::checker::Checker;
use crate::syntax::{ExprCollector, decorator_names};

// The sections of a definition. A method that is in none of them sorts after all of them.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Section {
    Nested,
    Constructor(usize),
    Factory,
    Dunder,
    Property,
}

impl Section {
    // The place of the section in the layout, as a rank and a rank within it.
    fn rank(self) -> (usize, usize) {
        match self {
            Self::Nested => (0, 0),
            Self::Constructor(index) => (2, index),
            Self::Factory => (3, 0),
            Self::Dunder => (4, 0),
            Self::Property => (5, 0),
        }
    }

    pub(super) fn noun(self) -> &'static str {
        match self {
            Self::Nested => "the nested classes",
            Self::Constructor(_) => "the constructors",
            Self::Factory => "the alternative constructors",
            Self::Dunder => "the dunder methods",
            Self::Property => "the properties",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Arrangement {
    // SMP201: entry points keep their order, so only helpers move.
    Helpers,
    // SMP203: entry points are sorted into sections, with their helpers.
    Sections,
}

// A definition that sits below a definition of a later section, with the first such definition above it.
pub(super) struct Inversion<'a> {
    pub(super) statement: usize,
    pub(super) range: TextRange,
    pub(super) name: &'a str,
    pub(super) anchor: usize,
    pub(super) anchor_name: &'a str,
    pub(super) section: Section,
    pub(super) anchor_section: Option<Section>,
}

#[derive(Clone, Copy)]
enum Kind<'a> {
    // A statement that is not a definition.
    Statement,
    // A definition, which is a method when it is in no section.
    Definition { name: &'a str, section: Option<Section> },
}

// The statements that move together: every `def` of one name, such as an overload set or a property and its setter,
// or a single statement that is not a definition.
struct Unit<'a> {
    members: Vec<usize>,
    kind: Kind<'a>,
    // A unit that a name pins, or that conflicts with a definition, keeps its place among the entry points.
    fixed: bool,
}

impl Unit<'_> {
    fn first(&self) -> usize {
        self.members[0]
    }

    fn last(&self) -> usize {
        self.members[self.members.len() - 1]
    }

    fn is_definition(&self) -> bool {
        matches!(self.kind, Kind::Definition { .. })
    }

    // The place of the unit in the layout: the nested classes, then the class attributes, then the definitions by section,
    // with the methods last.
    fn rank(&self) -> (usize, usize) {
        match self.kind {
            Kind::Statement => (1, 0),
            Kind::Definition { section, .. } => section.map_or((6, 0), Section::rank),
        }
    }

    fn key(&self) -> ((usize, usize), usize) {
        (self.rank(), self.first())
    }
}

pub(super) struct ClassOrder<'a> {
    units: Vec<Unit<'a>>,
    unit_of: Vec<usize>,
    // The range each statement reports at: its name, or the statement itself when it has none.
    ranges: Vec<TextRange>,
    owner: Vec<Option<usize>>,
    children: Vec<Vec<usize>>,
}

// The one order of a class body that SMP201 and SMP203 both read, so a fix of either leaves the other with nothing to
// report. Entry points are sorted into sections, and each helper follows its first caller. A statement that is not a
// definition is an entry point of its own, in section 2. Two statements keep their order when one reads or binds a
// name the other binds, so a constructor never moves past a statement that its defaults or decorators read.
pub(super) fn class_order<'a>(checker: &Checker, class: &'a ast::StmtClassDef) -> ClassOrder<'a> {
    let body = &class.body;
    let lazy = checker.semantic().annotations_are_lazy();
    let reads: Vec<FxHashSet<&str>> = body.iter().map(|stmt| creation_reads(stmt, lazy)).collect();
    let binds: Vec<FxHashSet<&str>> = body.iter().map(|stmt| bound_names(stmt).into_iter().collect()).collect();
    let (mut units, unit_of, by_name) = units_of(class, &alternative_constructors(class));
    let swaps = Swaps { reads: &reads, binds: &binds, unit_of: &unit_of };
    pin(&mut units, &swaps, &by_name, body);
    let users = users(checker, class, &units, &unit_of);
    let owner = owners_of_pinned(&mut units, &swaps, &users);
    let mut children = vec![Vec::new(); units.len()];
    for (unit, owner) in owner.iter().enumerate() {
        if let Some(owner) = *owner {
            children[owner].push(unit);
        }
    }
    for (owner, list) in children.iter_mut().enumerate() {
        list.sort_by_key(|&unit| users[unit].iter().filter(|(user, _)| *user == owner).map(|(_, at)| *at).min());
    }
    let ranges = body.iter().map(|stmt| definition_name(stmt).map_or(stmt.range(), Ranged::range)).collect();
    ClassOrder { units, unit_of, ranges, owner, children }
}

// The units of a class body: each `def` of one name with the others of its name, and each other statement on its own.
fn units_of<'a>(
    class: &'a ast::StmtClassDef,
    factories: &FxHashSet<&str>,
) -> (Vec<Unit<'a>>, Vec<usize>, FxHashMap<&'a str, usize>) {
    let mut units: Vec<Unit> = Vec::new();
    let mut unit_of = Vec::with_capacity(class.body.len());
    let mut by_name: FxHashMap<&str, usize> = FxHashMap::default();
    for (index, stmt) in class.body.iter().enumerate() {
        let unit = if let Some(name) = definition_name(stmt).map(ast::Identifier::as_str) {
            *by_name.entry(name).or_insert_with(|| {
                let kind = Kind::Definition { name, section: section(stmt, factories) };
                units.push(Unit { members: Vec::new(), kind, fixed: false });
                units.len() - 1
            })
        } else {
            units.push(Unit { members: Vec::new(), kind: Kind::Statement, fixed: false });
            units.len() - 1
        };
        units[unit].members.push(index);
        unit_of.push(unit);
    }
    (units, unit_of, by_name)
}

// Pins the units that keep their place: a definition a body reads while it runs, a statement that conflicts with a
// definition, the definitions the runtime finds by name, and the docstring.
fn pin(units: &mut [Unit], swaps: &Swaps, by_name: &FxHashMap<&str, usize>, body: &[Stmt]) {
    // A name the body reads while it runs pins the definition that binds it, since the reader needs the binding in
    // place. A definition reading its own name, as a property setter does, is not such a read.
    for (index, names_read) in swaps.reads.iter().enumerate() {
        for name in names_read {
            if let Some(&unit) = by_name.get(name)
                && swaps.unit_of[index] != unit
            {
                units[unit].fixed = true;
            }
        }
    }
    // A statement that is not a definition is fixed when it conflicts with a definition, and otherwise moves freely
    // among the entry points of its stretch.
    let conflicting: Vec<usize> = (0..units.len())
        .filter(|&unit| {
            !units[unit].is_definition()
                && units[unit].members.iter().any(|&member| {
                    (0..body.len()).any(|other| {
                        units[swaps.unit_of[other]].is_definition()
                            && swaps.unit_of[other] != unit
                            && !swaps.commutes(member, other)
                    })
                })
        })
        .collect();
    for unit in conflicting {
        units[unit].fixed = true;
    }
    // An enum's `auto()` reads `_generate_next_value_` from the class namespace rather than by name, and the enum rejects
    // the definition once a member above it has called `auto()`.
    if let Some(&unit) = by_name.get("_generate_next_value_") {
        units[unit].fixed = true;
    }
    // A docstring documents the class, so it stays first.
    if let Some(Stmt::Expr(ast::StmtExpr { value, .. })) = body.first()
        && value.is_string_literal_expr()
    {
        units[swaps.unit_of[0]].fixed = true;
    }
}

// The statements that read each definition of the class body, with the statement and the offset of each read.
fn users(
    checker: &Checker,
    class: &ast::StmtClassDef,
    units: &[Unit],
    unit_of: &[usize],
) -> Vec<Vec<(usize, TextSize)>> {
    let body = &class.body;
    let mut users: Vec<Vec<(usize, TextSize)>> = vec![Vec::new(); units.len()];
    for (unit, info) in units.iter().enumerate() {
        let Kind::Definition { name, .. } = info.kind else { continue };
        for reference in checker.semantic().member_references(class.range, name) {
            let reader = body.partition_point(|stmt| stmt.end() <= reference.start());
            if body.get(reader).is_some_and(|stmt| stmt.range().contains_range(*reference)) && unit_of[reader] != unit {
                users[unit].push((unit_of[reader], reference.start()));
            }
        }
    }
    users
}

// The owner of each unit, found again after each pinning until no unit needs pinning: pinning a helper or a split
// definition changes who owns what.
fn owners_of_pinned(units: &mut [Unit], swaps: &Swaps, users: &[Vec<(usize, TextSize)>]) -> Vec<Option<usize>> {
    loop {
        let mut pinned = false;
        for unit in 0..units.len() {
            if !units[unit].fixed && units[unit].is_definition() && swaps.splits(units, unit) {
                units[unit].fixed = true;
                pinned = true;
            }
        }
        let owner = owners(units, users);
        for (unit, owner) in owner.iter().enumerate() {
            if let Some(owner) = *owner
                && !swaps.can_move(units, unit, owner)
            {
                units[unit].fixed = true;
                pinned = true;
            }
        }
        if !pinned {
            break owner;
        }
    }
}

// The owner of each helper: the caller that comes first when the sections are sorted and each caller is followed by its
// helpers, depth first. Both arrangements pick the same caller, and a fix that sorts the sections leaves each helper's owner
// as it was. A helper that no entry point reaches, as in a pair of methods that only call each other, becomes an entry
// point itself. Constructors, statements that are not definitions and fixed units never move under a caller.
fn owners(units: &[Unit<'_>], users: &[Vec<(usize, TextSize)>]) -> Vec<Option<usize>> {
    let mut helper: Vec<bool> = units
        .iter()
        .zip(users)
        .map(|(unit, users)| {
            let constructor = matches!(unit.kind, Kind::Definition { section: Some(Section::Constructor(_)), .. });
            !(unit.fixed || !unit.is_definition() || constructor || users.is_empty())
        })
        .collect();
    let mut callees: Vec<Vec<(TextSize, usize)>> = vec![Vec::new(); units.len()];
    for (unit, users) in users.iter().enumerate() {
        for &(user, at) in users {
            callees[user].push((at, unit));
        }
    }
    for list in &mut callees {
        list.sort_unstable();
    }
    loop {
        let mut owner = vec![None; units.len()];
        for entry in entry_order(units, |unit| !helper[unit], Arrangement::Sections) {
            let mut stack = vec![entry];
            while let Some(current) = stack.pop() {
                let start = stack.len();
                for &(_, callee) in &callees[current] {
                    if helper[callee] && owner[callee].is_none() {
                        owner[callee] = Some(current);
                        stack.push(callee);
                    }
                }
                stack[start..].reverse();
            }
        }
        let unreached = (0..units.len())
            .filter(|&unit| helper[unit] && owner[unit].is_none())
            .min_by_key(|&unit| units[unit].key());
        match unreached {
            Some(unit) => helper[unit] = false,
            None => break owner,
        }
    }
}

// The entry points in the arrangement's order: a fixed unit keeps its place, and under `Arrangement::Sections` the
// units between two fixed units sort by section.
fn entry_order(units: &[Unit<'_>], is_entry: impl Fn(usize) -> bool, arrangement: Arrangement) -> Vec<usize> {
    let mut order = Vec::with_capacity(units.len());
    let mut segment = Vec::new();
    for unit in (0..units.len()).filter(|&unit| is_entry(unit)) {
        if units[unit].fixed {
            flush(units, &mut segment, arrangement, &mut order);
            order.push(unit);
        } else {
            segment.push(unit);
        }
    }
    flush(units, &mut segment, arrangement, &mut order);
    order
}

fn flush(units: &[Unit<'_>], segment: &mut Vec<usize>, arrangement: Arrangement, order: &mut Vec<usize>) {
    if arrangement == Arrangement::Sections {
        segment.sort_by_key(|&unit| units[unit].key());
    }
    order.append(segment);
}

// Which statements of a class body can swap with each other, from what each reads and binds.
struct Swaps<'r, 'a> {
    reads: &'r [FxHashSet<&'a str>],
    binds: &'r [FxHashSet<&'a str>],
    unit_of: &'r [usize],
}

impl Swaps<'_, '_> {
    // Whether a helper can take its owner's place: no statement between them conflicts with it.
    fn can_move(&self, units: &[Unit], unit: usize, owner: usize) -> bool {
        let info = &units[unit];
        let low = info.first().min(units[owner].first());
        let high = info.last().max(units[owner].last());
        (low..=high)
            .all(|other| self.unit_of[other] == unit || info.members.iter().all(|&member| self.commutes(member, other)))
    }

    // Whether a definition's members cannot be contiguous: a statement among them is fixed, or conflicts with a member.
    fn splits(&self, units: &[Unit], unit: usize) -> bool {
        let info = &units[unit];
        (info.first() + 1..info.last()).any(|other| {
            self.unit_of[other] != unit
                && (units[self.unit_of[other]].fixed
                    || info.members.iter().any(|&member| !self.commutes(member, other)))
        })
    }

    // Whether two statements can swap: neither reads a name the other binds, and they bind no name in common.
    fn commutes(&self, first: usize, second: usize) -> bool {
        self.reads[first].is_disjoint(&self.binds[second])
            && self.reads[second].is_disjoint(&self.binds[first])
            && self.binds[first].is_disjoint(&self.binds[second])
    }
}

impl<'a> ClassOrder<'a> {
    // The statement order the arrangement asks for, as a permutation of the body's statements.
    pub(super) fn expected(&self, arrangement: Arrangement) -> Vec<usize> {
        let mut order = Vec::with_capacity(self.unit_of.len());
        for unit in entry_order(&self.units, |unit| self.owner[unit].is_none(), arrangement) {
            self.emit(unit, &mut order);
        }
        order
    }

    // The owning statement of each statement that is a helper, `None` for the rest.
    pub(super) fn helper_owners(&self) -> Vec<Option<usize>> {
        self.unit_of.iter().map(|&unit| self.owner[unit].map(|owner| self.units[owner].first())).collect()
    }

    // The definitions that sit below a definition of a later section, with the first such definition above them.
    pub(super) fn inversions(&self) -> Vec<Inversion<'a>> {
        let entries: Vec<(usize, &'a str, Option<Section>)> = (0..self.units.len())
            .filter_map(|unit| match self.units[unit].kind {
                Kind::Definition { name, section } if self.owner[unit].is_none() => Some((unit, name, section)),
                Kind::Definition { .. } | Kind::Statement => None,
            })
            .collect();
        let mut inversions = Vec::new();
        for (position, &(unit, name, section)) in entries.iter().enumerate() {
            let Some(section) = section else { continue };
            let Some(&(anchor, anchor_name, anchor_section)) = entries[..position]
                .iter()
                .find(|(_, _, earlier)| earlier.is_none_or(|earlier| earlier.rank() > section.rank()))
            else {
                continue;
            };
            for &member in &self.units[unit].members {
                inversions.push(Inversion {
                    statement: member,
                    range: self.ranges[member],
                    name,
                    anchor: self.units[anchor].first(),
                    anchor_name,
                    section,
                    anchor_section,
                });
            }
        }
        inversions
    }

    // A unit, then the helpers that follow it depth first, as each helper follows its first caller.
    fn emit(&self, unit: usize, order: &mut Vec<usize>) {
        let mut stack = vec![unit];
        while let Some(current) = stack.pop() {
            order.extend(self.units[current].members.iter().copied());
            stack.extend(self.children[current].iter().rev());
        }
    }
}

// The section of a definition; `None` for a method, which is in no section.
fn section(stmt: &Stmt, factories: &FxHashSet<&str>) -> Option<Section> {
    let Stmt::FunctionDef(function) = stmt else { return Some(Section::Nested) };
    let name = function.name.as_str();
    if let Some(rank) = ["__init_subclass__", "__new__", "__init__", "__post_init__"]
        .iter()
        .position(|constructor| *constructor == name)
    {
        Some(Section::Constructor(rank))
    } else if factories.contains(name) {
        Some(Section::Factory)
    } else if name.len() > 4 && name.starts_with("__") && name.ends_with("__") {
        Some(Section::Dunder)
    } else if decorator_names(&function.decorator_list)
        .iter()
        .any(|name| matches!(*name, "property" | "cached_property" | "setter" | "getter" | "deleter"))
    {
        Some(Section::Property)
    } else {
        None
    }
}

// The names a statement reads while the class body runs: the parts of a `def` that run when it is defined, or the whole
// of any other statement.
fn creation_reads(stmt: &Stmt, lazy: bool) -> FxHashSet<&str> {
    let mut reads = ExprCollector::new(|expr| {
        expr.as_name_expr().filter(|name| name.ctx == ExprContext::Load).map(|name| name.id.as_str())
    });
    match stmt {
        Stmt::FunctionDef(function) => {
            for expr in definition_time_exprs(function) {
                reads.visit_expr(expr);
            }
            // Decorators read the annotations they decorate even under lazy annotations, as SMP201 assumes.
            if !lazy || !function.decorator_list.is_empty() {
                for expr in signature_annotations(function) {
                    reads.visit_expr(expr);
                }
            }
        }
        other => reads.visit_stmt(other),
    }
    reads.found.into_iter().collect()
}
