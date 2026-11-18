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
    // The strongly connected component the file is in, numbered per run: files of one cycle share it.
    pub(super) component: usize,
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
    graph.retain(|file, imports| {
        let home = component[file];
        imports.retain(|(target, _)| component[target] == home);
        sizes[&home] > 1
    });
    let mut incoming = FxHashMap::default();
    for &(target, _) in graph.values().flatten() {
        *incoming.entry(target).or_insert(0usize) += 1;
    }
    let unbranched = |file: &FileId| incoming[file] == 1 && graph[file].len() == 1;
    let mut cycles = FxHashMap::default();
    for (&file, imports) in &graph {
        if cycles.contains_key(&file) {
            continue;
        }
        let mut path = shortest_path(&graph, imports[0].0, file);
        path.rotate_right(1);
        // Along a stretch with one way in and one way out, every module must take the same return path.
        let (prefix, suffix) = if unbranched(&file) {
            let prefix = path.iter().take_while(|id| unbranched(id)).count();
            (prefix, path[prefix..].iter().rev().take_while(|id| unbranched(id)).count())
        } else {
            (1, 0)
        };
        for at in (0..prefix).chain(path.len() - suffix..path.len()) {
            let member = path[at];
            let chain = path[at..].iter().chain(&path[..at]).copied().take(SHOWN).chain([member]).collect();
            cycles.insert(
                member,
                Cycle {
                    range: graph[&member][0].1,
                    chain,
                    omitted: path.len().saturating_sub(SHOWN),
                    component: component[&member],
                },
            );
        }
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

fn shortest_path(graph: &RuntimeGraph, from: FileId, to: FileId) -> Vec<FileId> {
    let mut previous: FxHashMap<FileId, FileId> = FxHashMap::default();
    let mut seen: FxHashSet<FileId> = FxHashSet::from_iter([from]);
    let mut queue = VecDeque::from([from]);
    while let Some(file) = queue.pop_front() {
        if file == to {
            break;
        }
        for (next, _) in &graph[&file] {
            if seen.insert(*next) {
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
