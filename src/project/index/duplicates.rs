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
// longest single copy spans.
#[derive(Clone, PartialEq)]
pub struct DuplicateRun {
    pub(super) range: TextRange,
    pub(super) others: Vec<(PathBuf, u32)>,
    pub(super) statements: usize,
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
}

impl CacheKey for DuplicateRun {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        range_key(self.range(), state);
        self.others().cache_key(state);
        self.statements().cache_key(state);
    }
}

// A window of statements, by its file and the position of its first statement among the file's statements.
type Place = (FileId, usize);

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
    // The starts of the windows each file shares with each other file, by the offset between the two copies.
    let mut shared: FxHashMap<FileId, BTreeMap<(FileId, isize), Vec<usize>>> = FxHashMap::default();
    for places in groups.values() {
        for &(file, start) in places {
            for &(other, other_start) in places.iter().filter(|(other, _)| *other != file) {
                let offset = start.cast_signed() - other_start.cast_signed();
                shared.entry(file).or_default().entry((other, offset)).or_default().push(start);
            }
        }
    }
    shared
        .into_iter()
        .map(|(file, copies)| {
            let blocks = blocks(&copies, statements[&file], &statements, paths, window);
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
    own: &[Statement],
    statements: &FxHashMap<FileId, &[Statement]>,
    paths: &FileIds,
    window: usize,
) -> Vec<DuplicateRun> {
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
            // Each other file once, where its first copy in the block starts.
            let mut places: BTreeMap<PathBuf, u32> = BTreeMap::new();
            for (other, line) in block.others {
                let line_of = places.entry(paths.path(other).to_path_buf()).or_insert(line);
                *line_of = (*line_of).min(line);
            }
            DuplicateRun { range: *range, others: places.into_iter().collect(), statements: block.longest }
        })
        .collect()
}

// Overlapping copies of one file: its first and last statement, the statements of its longest single copy, and the other
// file and start line of each copy.
struct Block {
    first: usize,
    last: usize,
    longest: usize,
    others: Vec<(FileId, u32)>,
}
