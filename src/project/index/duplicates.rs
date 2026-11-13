use std::collections::BTreeMap;
use std::hash::Hasher;
use std::path::PathBuf;

use rayon::prelude::*;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_text_size::{TextRange, TextSize};
use rustc_hash::{FxHashMap, FxHashSet, FxHasher};

use super::range_key;
use super::{FileId, FileIds};
use crate::project::FileFacts;
use crate::project::facts::{Statement, statement_texts};

#[derive(Clone, PartialEq)]
pub struct DuplicateRun {
    pub(super) range: TextRange,
    pub(super) others: Vec<(PathBuf, u32)>,
    pub(super) statements: usize,
    pub(super) sharers: usize,
}

impl DuplicateRun {
    pub fn range(&self) -> TextRange {
        self.range
    }

    pub fn others(&self) -> &[(PathBuf, u32)] {
        &self.others
    }

    pub fn statements(&self) -> usize {
        self.statements
    }

    pub fn sharers(&self) -> usize {
        self.sharers
    }
}

impl CacheKey for DuplicateRun {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        range_key(self.range(), state);
        self.others().cache_key(state);
        self.statements().cache_key(state);
        self.sharers().cache_key(state);
    }
}

type Place = (FileId, usize);

// Only repeats in another file count, as pylint's duplicate check compares files with each other.
// Windows match by the hash of their statements' hashes first, and then by the statements' token texts, so a hash
// collision never reports code that differs.
pub(super) fn duplicate_runs(
    files: &[(FileId, &FileFacts)],
    paths: &FileIds,
    window: usize,
) -> FxHashMap<FileId, Vec<DuplicateRun>> {
    let statements: FxHashMap<FileId, &[Statement]> =
        files.iter().map(|(file, facts)| (*file, facts.statements.as_slice())).collect();
    let mut by_hash: FxHashMap<u64, Vec<Place>> = FxHashMap::default();
    for (file, _) in files {
        for (start, run) in statements[file].windows(window).enumerate() {
            if run[0].body == run[window - 1].body {
                let mut hasher = FxHasher::default();
                for statement in run {
                    hasher.write_u64(statement.hash);
                }
                by_hash.entry(hasher.finish()).or_default().push((*file, start));
            }
        }
    }
    by_hash.retain(|_, places| places.iter().any(|(file, _)| *file != places[0].0));
    let texts = texts(&by_hash, &statements, paths, window);
    let mut numbers: FxHashMap<&[u8], usize> = FxHashMap::default();
    let mut groups: FxHashMap<Vec<usize>, Vec<Place>> = FxHashMap::default();
    for places in by_hash.values() {
        for &(file, start) in places {
            let key: Option<Vec<usize>> = (start..start + window)
                .map(|at| {
                    let text = texts.get(&(file, at))?;
                    let next = numbers.len();
                    Some(*numbers.entry(text.as_slice()).or_insert(next))
                })
                .collect();
            if let Some(key) = key {
                groups.entry(key).or_default().push((file, start));
            }
        }
    }
    let mut sets: FxHashMap<Vec<FileId>, usize> = FxHashMap::default();
    let mut set_files: Vec<Vec<FileId>> = Vec::new();
    let mut windows: FxHashMap<FileId, Vec<(usize, usize)>> = FxHashMap::default();
    let mut shared: FxHashMap<FileId, BTreeMap<(FileId, isize), Vec<usize>>> = FxHashMap::default();
    for mut places in groups.into_values() {
        places.sort_unstable();
        let mut members: Vec<FileId> = places.iter().map(|(file, _)| *file).collect();
        members.dedup();
        let next = set_files.len();
        let set = *sets.entry(members.clone()).or_insert(next);
        if set == next {
            set_files.push(members);
        }
        for &(file, start) in &places {
            windows.entry(file).or_default().push((start, set));
        }
        let chunks: Vec<&[Place]> = places.chunk_by(|(left, _), (right, _)| left == right).collect();
        let heads: Vec<Place> = chunks.iter().map(|copies| copies[0]).collect();
        for copies in chunks {
            let file = copies[0].0;
            // Pairs each window with a few partners, so the pairs grow with the files rather than with every pair of
            // them. The blocks still count every file that repeats a window.
            let partners: Vec<Place> = heads.iter().filter(|(other, _)| *other != file).take(3).copied().collect();
            for &(place_file, start) in copies {
                for &(other, other_start) in &partners {
                    let offset = start.cast_signed() - other_start.cast_signed();
                    shared.entry(place_file).or_default().entry((other, offset)).or_default().push(start);
                }
            }
        }
    }
    for list in windows.values_mut() {
        list.sort_unstable();
    }
    shared
        .into_iter()
        .map(|(file, copies)| (file, blocks(&copies, file, &statements, paths, window, &windows[&file], &set_files)))
        .collect()
}

// A file that changed since its statements were recorded has none.
fn texts(
    by_hash: &FxHashMap<u64, Vec<Place>>,
    statements: &FxHashMap<FileId, &[Statement]>,
    paths: &FileIds,
    window: usize,
) -> FxHashMap<Place, Vec<u8>> {
    let mut wanted: FxHashMap<FileId, FxHashSet<usize>> = FxHashMap::default();
    for &(file, start) in by_hash.values().flatten() {
        wanted.entry(file).or_default().extend(start..start + window);
    }
    wanted
        .into_par_iter()
        .filter_map(|(file, wanted)| {
            let texts = statement_texts(paths.path(file), &wanted, statements[&file])?;
            Some(texts.into_iter().map(move |(at, text)| ((file, at), text)))
        })
        .flatten_iter()
        .collect()
}

fn blocks(
    copies: &BTreeMap<(FileId, isize), Vec<usize>>,
    file: FileId,
    statements: &FxHashMap<FileId, &[Statement]>,
    paths: &FileIds,
    window: usize,
    windows: &[(usize, usize)],
    sets: &[Vec<FileId>],
) -> Vec<DuplicateRun> {
    let own = statements[&file];
    let mut spans: Vec<(usize, usize, FileId, u32)> = Vec::new();
    for (&(other, offset), starts) in copies {
        let mut starts = starts.clone();
        starts.sort_unstable();
        starts.dedup();
        let mut first = starts[0];
        for (at, &start) in starts.iter().enumerate() {
            if starts.get(at + 1) != Some(&(start + 1)) {
                let other_start = (first.cast_signed() - offset).cast_unsigned();
                spans.push((first, start + window - 1, other, statements[&other][other_start].line));
                first = starts.get(at + 1).copied().unwrap_or_default();
            }
        }
    }
    spans.sort_unstable_by_key(|(first, last, ..)| (*first, *last));
    let mut blocks: Vec<Block> = Vec::new();
    for (first, last, other, line) in spans {
        match blocks.last_mut() {
            Some(block) if first <= block.last => {
                block.last = block.last.max(last);
                block.longest = block.longest.max(last - first + 1);
                block.others.push((other, line));
            }
            _ => blocks.push(Block { first, last, longest: last - first + 1, others: vec![(other, line)] }),
        }
    }
    let ranges: Vec<TextRange> = blocks
        .iter()
        .map(|block| TextRange::new(TextSize::new(own[block.first].start), TextSize::new(own[block.last].end)))
        .collect();
    blocks
        .into_iter()
        .zip(&ranges)
        .filter(|(_, range)| !ranges.iter().any(|outer| outer != *range && outer.contains_range(**range)))
        .map(|(block, range)| {
            let sharers = sharers(file, &block, windows, sets, window);
            let mut places: BTreeMap<PathBuf, u32> = BTreeMap::new();
            for (other, line) in block.others {
                let line_of = places.entry(paths.path(other).to_path_buf()).or_insert(line);
                *line_of = (*line_of).min(line);
            }
            DuplicateRun { range: *range, others: places.into_iter().collect(), statements: block.longest, sharers }
        })
        .collect()
}

struct Block {
    first: usize,
    last: usize,
    longest: usize,
    others: Vec<(FileId, u32)>,
}

// A set that several windows of the block share is counted once.
fn sharers(file: FileId, block: &Block, windows: &[(usize, usize)], sets: &[Vec<FileId>], window: usize) -> usize {
    let first = windows.partition_point(|(start, _)| *start < block.first);
    let end = windows.partition_point(|(start, _)| *start <= block.last + 1 - window);
    let mut picked: Vec<usize> = windows[first..end].iter().map(|(_, set)| *set).collect();
    picked.sort_unstable();
    picked.dedup();
    if let [set] = picked[..] {
        return sets[set].len() - 1;
    }
    let mut others: FxHashSet<FileId> = picked.iter().flat_map(|set| sets[*set].iter().copied()).collect();
    others.remove(&file);
    others.len()
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use tempfile::TempDir;
    use test_case::test_case;

    use super::duplicate_runs;
    use crate::project::FileFacts;
    use crate::project::facts::Parts;
    use crate::project::index::FileIds;

    // The statements after a block start at the dedent that closes it. A file that no longer holds the statements its
    // facts record has no texts to compare, so its windows match nothing.
    #[test_case(false, 2 ; "both_files_read")]
    #[test_case(true, 0 ; "file_gone")]
    fn windows_match_by_their_texts(removed: bool, files_with_runs: usize) {
        let source = "if FLAG:\n    first = 1\n    second = 2\nthird = 3\nfourth = 4\n";
        let root = TempDir::new().expect("create a temporary directory");
        let mut ids = Arc::new(FileIds::default());
        let files: Vec<_> = ["a.py", "b.py"]
            .into_iter()
            .map(|name| {
                let path = root.path().join(name);
                std::fs::write(&path, source).expect("write the module");
                let parts = Parts::default().with(Parts::STATEMENTS);
                let facts = FileFacts::collect(&path, source, None, None, parts).expect("facts");
                (FileIds::intern(&mut ids, &path), facts)
            })
            .collect();
        if removed {
            std::fs::remove_file(root.path().join("b.py")).expect("remove the module");
        }
        let files: Vec<_> = files.iter().map(|(id, facts)| (*id, facts)).collect();
        assert_eq!(duplicate_runs(&files, &ids, 2).len(), files_with_runs);
    }
}
