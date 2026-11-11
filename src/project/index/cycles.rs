use std::collections::VecDeque;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_text_size::TextRange;
use rustc_hash::{FxHashMap, FxHashSet};

use super::range_key;
use super::{FileId, FileIds};

// A ring of thousands of modules would otherwise name every one of them in each of its reports.
const SHOWN: usize = 8;

#[derive(Clone, PartialEq)]
pub struct Cycle {
    pub(super) range: TextRange,
    pub(super) chain: Vec<FileId>,
    pub(super) omitted: usize,
}

impl Cycle {
    pub fn range(&self) -> TextRange {
        self.range
    }

    pub fn chain(&self) -> &[FileId] {
        &self.chain
    }

    pub fn omitted(&self) -> usize {
        self.omitted
    }

    // Ids are numbered per run, so the key hashes the paths, which a cached report must match in the next run.
    pub(super) fn cache_key_with(&self, files: &FileIds, state: &mut CacheKeyHasher) {
        range_key(self.range(), state);
        self.omitted.cache_key(state);
        self.chain.iter().map(|&id| files.path(id)).collect::<Vec<_>>().cache_key(state);
    }
}

pub(super) type RuntimeGraph = FxHashMap<FileId, Vec<(FileId, TextRange)>>;

pub(super) fn cycles(mut graph: RuntimeGraph) -> FxHashMap<FileId, Cycle> {
    for imports in graph.values_mut() {
        imports.sort_by_key(|(_, range)| range.start());
    }
    let component = components(&graph);
    let mut sizes: FxHashMap<usize, usize> = FxHashMap::default();
    for component in component.values() {
        *sizes.entry(*component).or_default() += 1;
    }
    // A component where every file has one import into it is a single ring, so the path from a file is the one its
    // imports follow, and the search for it is not needed.
    let mut rings: FxHashMap<usize, usize> = FxHashMap::default();
    for (&file, imports) in &graph {
        let home = component[&file];
        if sizes[&home] >= 2 && inside(imports, &component, home).count() == 1 {
            *rings.entry(home).or_default() += 1;
        }
    }
    let mut cycles = FxHashMap::default();
    for (&file, imports) in &graph {
        // Every file with imports is in a component, and one in a component of two or more files imports into it.
        let home = component[&file];
        let size = sizes[&home];
        if size < 2 {
            continue;
        }
        let &(first, range) =
            inside(imports, &component, home).next().expect("a file in a cycle imports a file of its component");
        let (chain, omitted) = if rings.get(&home) == Some(&size) {
            ring_chain(&graph, &component, home, file, size)
        } else {
            let mut chain = vec![file];
            chain.extend(shortest_path(&graph, &component, home, first, file));
            // The path ends on `file`, which the chain closes on once more after the files it keeps.
            chain.pop();
            let omitted = chain.len().saturating_sub(SHOWN);
            chain.truncate(SHOWN);
            // Without this, each cycle of a ring keeps the allocation of the whole path it was cut from.
            chain.shrink_to_fit();
            chain.push(file);
            (chain, omitted)
        };
        cycles.insert(file, Cycle { range, chain, omitted });
    }
    cycles
}

// Iterative, so a long chain of imports does not exhaust the stack.
fn components(graph: &RuntimeGraph) -> FxHashMap<FileId, usize> {
    let mut order: FxHashMap<FileId, usize> = FxHashMap::default();
    let mut low: FxHashMap<FileId, usize> = FxHashMap::default();
    let mut on_stack: FxHashSet<FileId> = FxHashSet::default();
    let mut stack: Vec<FileId> = Vec::new();
    let mut component: FxHashMap<FileId, usize> = FxHashMap::default();
    let mut components = 0;
    for &root in graph.keys() {
        if order.contains_key(&root) {
            continue;
        }
        let mut frames: Vec<(FileId, usize)> = vec![(root, 0)];
        order.insert(root, order.len());
        low.insert(root, order[&root]);
        stack.push(root);
        on_stack.insert(root);
        while let Some((file, next)) = frames.last_mut() {
            let file = *file;
            let targets = graph.get(&file).map_or(&[][..], Vec::as_slice);
            if let Some((target, _)) = targets.get(*next) {
                let target = *target;
                *next += 1;
                if let Some(&reached) = order.get(&target) {
                    if on_stack.contains(&target) {
                        let lowest = low.get_mut(&file).expect("a visited file has a low link");
                        *lowest = (*lowest).min(reached);
                    }
                } else {
                    order.insert(target, order.len());
                    low.insert(target, order[&target]);
                    stack.push(target);
                    on_stack.insert(target);
                    frames.push((target, 0));
                }
                continue;
            }
            frames.pop();
            if low[&file] == order[&file] {
                while let Some(member) = stack.pop() {
                    on_stack.remove(&member);
                    component.insert(member, components);
                    if member == file {
                        break;
                    }
                }
                components += 1;
            }
            if let Some((parent, _)) = frames.last() {
                let lowest = low[&file];
                let parent_low = low.get_mut(parent).expect("a visited file has a low link");
                *parent_low = (*parent_low).min(lowest);
            }
        }
    }
    component
}

fn inside<'a>(
    imports: &'a [(FileId, TextRange)],
    component: &'a FxHashMap<FileId, usize>,
    home: usize,
) -> impl Iterator<Item = &'a (FileId, TextRange)> {
    imports.iter().filter(move |(target, _)| component.get(target) == Some(&home))
}

fn ring_chain(
    graph: &RuntimeGraph,
    component: &FxHashMap<FileId, usize>,
    home: usize,
    file: FileId,
    size: usize,
) -> (Vec<FileId>, usize) {
    let mut chain = vec![file];
    let mut at = file;
    while chain.len() < size.min(SHOWN) {
        at = inside(&graph[&at], component, home).next().expect("a file in a ring imports a file of it").0;
        chain.push(at);
    }
    chain.push(file);
    (chain, size.saturating_sub(SHOWN))
}

fn shortest_path(
    graph: &RuntimeGraph,
    component: &FxHashMap<FileId, usize>,
    home: usize,
    from: FileId,
    to: FileId,
) -> Vec<FileId> {
    let mut previous: FxHashMap<FileId, FileId> = FxHashMap::default();
    let mut seen: FxHashSet<FileId> = FxHashSet::from_iter([from]);
    let mut queue = VecDeque::from([from]);
    while let Some(file) = queue.pop_front() {
        if file == to {
            break;
        }
        for (next, _) in graph.get(&file).into_iter().flatten() {
            if component.get(next) == Some(&home) && seen.insert(*next) {
                previous.insert(*next, file);
                queue.push_back(*next);
            }
        }
    }
    let mut path = vec![to];
    while let Some(file) = previous.get(path.last().expect("the path holds `to`")) {
        path.push(*file);
    }
    path.reverse();
    path
}
