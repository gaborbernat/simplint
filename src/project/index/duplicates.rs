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

// A block of statements that other files repeat: the place each other file's copy starts, and how many statements the
// longest single copy spans. `others` names some of the files that repeat the block, and `sharers` counts all of them.
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

// A window of statements, by its file and the position of its first statement among the file's statements.
type Place = (FileId, usize);

// A window repeated in many files pairs each place with this many other files, one place in each, so the pairs grow with
// the files rather than with every pair of them. The blocks still count every file that repeats a window.
const PARTNERS: usize = 3;

// The duplicate blocks of every file, made of windows of `window` statements of one body that another file holds too.
// Only repeats in another file count, as pylint's duplicate check compares files with each other.
//
// Windows match by the hash of their statements' hashes first, and then by the statements' token texts, so a hash
// collision never reports code that differs. The windows a file shares with one other file at a fixed offset, one
// statement apart, form one copy; the copies of a file that overlap form one block, and a block inside another, as the
// body of a function the other block holds, is left to the outer one.
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
    // Each window by the texts of its statements, numbered as they first appear.
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
    // The files each window's group repeats together, numbered as they first appear, and the windows of each file with
    // the number of its group.
    let mut sets: FxHashMap<Vec<FileId>, usize> = FxHashMap::default();
    let mut set_files: Vec<Vec<FileId>> = Vec::new();
    let mut windows: FxHashMap<FileId, Vec<(usize, usize)>> = FxHashMap::default();
    // The starts of the windows each file shares with each other file, by the offset between the two copies.
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
            let partners: Vec<Place> =
                heads.iter().filter(|(other, _)| *other != file).take(PARTNERS).copied().collect();
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
        .map(|(file, copies)| {
            let blocks = blocks(&copies, file, &statements, paths, window, &windows[&file], &set_files);
            (file, blocks)
        })
        .collect()
}

// The token texts of the statements of the windows that hash alike in more than one file, by file and position. A file
// that changed since its statements were recorded has none.
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

// The blocks of a file from the windows it shares with each other file at each offset.
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
    // Each copy as its first and last statement, the other file and the line its copy starts on.
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
            // Each other file once, where its first copy in the block starts.
            let mut places: BTreeMap<PathBuf, u32> = BTreeMap::new();
            for (other, line) in block.others {
                let line_of = places.entry(paths.path(other).to_path_buf()).or_insert(line);
                *line_of = (*line_of).min(line);
            }
            DuplicateRun { range: *range, others: places.into_iter().collect(), statements: block.longest, sharers }
        })
        .collect()
}

// How many other files repeat the statements of a block: the files of each set of windows the block covers, less the
// file itself. A set several windows of the block share is counted once.
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

// Overlapping copies of one file: its first and last statement, the statements of its longest single copy, and the other
// file and start line of each copy.
struct Block {
    first: usize,
    last: usize,
    longest: usize,
    others: Vec<(FileId, u32)>,
}
