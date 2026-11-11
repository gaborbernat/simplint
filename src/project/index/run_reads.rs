use std::hash::Hasher;

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use ruff_cache::{CacheKey, CacheKeyHasher};

use crate::project::computed_reads::ComputedReads;
use crate::project::{DocReferences, FileFacts};

use super::files::FileId;
use super::reach::unqualified;
use super::writes::Writes;
use super::{every, every_in};

// The names the files of the run list, the writes they make and what they read in common, which every file's answers
// depend on. `digest` hashes them for the cache, and `changed` names the ones that differ between two indexes.
pub(in crate::project) struct RunWide {
    pub(super) reads: RunReads,
    pub(super) assigned_attributes: FxHashSet<String>,
    pub(super) module_reads: FxHashSet<String>,
    pub(super) base_classes: FxHashSet<String>,
    // The fixtures test modules request by parameter, `usefixtures` or `getfixturevalue`, so pytest may use them.
    pub(super) fixture_requests: FxHashSet<String>,
    // The names code the index cannot follow reads: those a file imports or reads from a module that resolves to no file
    // of the run, and those its doctest examples use.
    pub(super) opaque_names: FxHashSet<String>,
    // The attributes the files read under computed names, which may reach any attribute or those with a prefix.
    pub(super) computed_reads: ComputedReads,
    pub(super) writes: Writes,
}

pub(super) fn run_names(
    indexed: &[(FileId, &FileFacts)],
    test_modules: &FxHashSet<FileId>,
    reads: RunReads,
    opaque_names: FxHashSet<String>,
    outer_assigned: &FxHashSet<String>,
) -> RunWide {
    let ((assigned_attributes, module_reads, base_classes), (fixture_requests, (computed_reads, writes))) = rayon::join(
        || {
            (
                every(indexed, |facts| &facts.assigned_attributes),
                every(indexed, |facts| &facts.module_reads),
                every(indexed, |facts| &facts.base_classes),
            )
        },
        || {
            rayon::join(
                || fixture_requests(indexed, test_modules),
                || rayon::join(|| computed_reads(indexed), || Writes::collect(indexed)),
            )
        },
    );
    // A store to the name in another project still makes it assigned here, as a store in this one does.
    let assigned_attributes: FxHashSet<String> =
        assigned_attributes.into_iter().chain(outer_assigned.iter().cloned()).collect();
    RunWide {
        reads,
        assigned_attributes,
        module_reads,
        base_classes,
        fixture_requests,
        opaque_names,
        computed_reads,
        writes,
    }
}

// The fixtures the test modules request, and those the tests and fixtures of any other file request.
fn fixture_requests(files: &[(FileId, &FileFacts)], test_modules: &FxHashSet<FileId>) -> FxHashSet<String> {
    let mut requests = every_in(files, |id| test_modules.contains(&id), |facts| &facts.fixture_requests);
    requests.extend(every(files, |facts| &facts.pytest_requests));
    requests
}

// The attributes the files read under names the program computes.
fn computed_reads(files: &[(FileId, &FileFacts)]) -> ComputedReads {
    let mut reads = ComputedReads::default();
    for (_, facts) in files {
        reads.extend(&facts.computed_reads);
    }
    reads
}

// What the files of the run read in common: whether any reads docstrings or a class's name off an object, so the names
// of the run's code may be read, and the number of files each word of a string literal appears in, which a rename must
// not change.
#[derive(Default, PartialEq)]
pub(super) struct RunReads {
    pub(super) docstrings: bool,
    pub(super) class_names: bool,
    // The names whose `__name__` or `__qualname__` some file of the run reads.
    pub(super) reflected: FxHashSet<String>,
    pub(super) string_words: FxHashMap<String, u32>,
}

impl RunReads {
    pub(super) fn collect(indexed: &[(FileId, &FileFacts)]) -> Self {
        Self {
            docstrings: indexed.iter().any(|(_, facts)| facts.reads_docstrings),
            class_names: indexed.iter().any(|(_, facts)| facts.reads_class_names),
            reflected: indexed.iter().flat_map(|(_, facts)| facts.reflected_names.iter().cloned()).collect(),
            string_words: string_word_counts(indexed),
        }
    }
}

// The number of files each word appears in. The files are counted in parallel by borrowed word, so each distinct word
// is copied once rather than once per file that holds it.
fn string_word_counts(indexed: &[(FileId, &FileFacts)]) -> FxHashMap<String, u32> {
    let counts = indexed
        .par_iter()
        .fold(FxHashMap::default, |mut counts: FxHashMap<&str, u32>, (_, facts)| {
            for word in facts.string_words.lines() {
                *counts.entry(word).or_default() += 1;
            }
            counts
        })
        .reduce(FxHashMap::default, |first, second| {
            let (mut larger, smaller) = if first.len() < second.len() { (second, first) } else { (first, second) };
            for (word, count) in smaller {
                *larger.entry(word).or_default() += count;
            }
            larger
        });
    counts.into_iter().map(|(word, count)| (word.to_owned(), count)).collect()
}

impl RunWide {
    // What the run's attribute writes, module reads and base classes are, and whether it reads docstrings: the names
    // of the whole run that every file's answers depend on. It is taken before the external sets are merged in.
    pub(super) fn digest(&self, docs: &DocReferences) -> u64 {
        let mut hasher = CacheKeyHasher::new();
        self.reads.docstrings.cache_key(&mut hasher);
        self.reads.class_names.cache_key(&mut hasher);
        self.reads.reflected.cache_key(&mut hasher);
        docs.cache_key(&mut hasher);
        self.computed_reads.cache_key(&mut hasher);
        string_words_key(&self.reads.string_words).cache_key(&mut hasher);
        // Each set is hashed behind its length, so a name moving from one set to the next changes the digest.
        for names in [
            &self.assigned_attributes,
            &self.module_reads,
            &self.base_classes,
            &self.fixture_requests,
            &self.opaque_names,
            &self.writes.untyped,
            &self.writes.dict_stores,
            &self.writes.unslottable_bases,
            &self.writes.stateful_bases,
        ] {
            names.cache_key(&mut hasher);
        }
        self.writes.on_class.cache_key(&mut hasher);
        self.writes.class_attributes.cache_key(&mut hasher);
        self.writes.pickles_low_protocol.cache_key(&mut hasher);
        hasher.finish()
    }

    // The words whose count of files differs between the two indexes, or that only one holds.
    fn string_word_changes<'p>(&'p self, other: &'p Self) -> impl Iterator<Item = &'p str> {
        let (own, others) = (&self.reads.string_words, &other.reads.string_words);
        let counts =
            own.iter().filter(|(word, count)| others.get(*word) != Some(*count)).map(|(word, _)| word.as_str());
        let only_other = others.keys().filter(|word| !own.contains_key(*word));
        counts.chain(only_other.map(String::as_str))
    }

    // The attribute, module read and base class names one index has and the other lacks, whose answers differ between
    // the two, and the names of the classes whose attribute writes, or subclasses that a slotted base breaks, differ,
    // which a dataclass of that name, or a namesake of it, reads. Low protocol pickling concerns every dataclass, so its
    // change names `dataclass`.
    pub(super) fn changed<'p>(&'p self, other: &'p Self) -> FxHashSet<&'p str> {
        let (writes, others) = (&self.writes, &other.writes);
        let classes = writes
            .on_class
            .iter()
            .filter(|(class, attributes)| others.on_class.get(*class) != Some(*attributes))
            .chain(others.on_class.iter().filter(|(class, _)| !writes.on_class.contains_key(*class)))
            .map(|(class, _)| class)
            .chain(writes.dict_stores.symmetric_difference(&others.dict_stores))
            .chain(writes.unslottable_bases.symmetric_difference(&others.unslottable_bases))
            .chain(writes.stateful_bases.symmetric_difference(&others.stateful_bases))
            .map(|class| unqualified(class));
        let pickling = (writes.pickles_low_protocol != others.pickles_low_protocol).then_some("dataclass");
        self.assigned_attributes
            .symmetric_difference(&other.assigned_attributes)
            .chain(self.module_reads.symmetric_difference(&other.module_reads))
            .chain(self.base_classes.symmetric_difference(&other.base_classes))
            .chain(self.fixture_requests.symmetric_difference(&other.fixture_requests))
            .chain(self.opaque_names.symmetric_difference(&other.opaque_names))
            .chain(writes.untyped.symmetric_difference(&others.untyped))
            .map(String::as_str)
            .chain(classes)
            .chain(pickling)
            .chain(self.string_word_changes(other))
            .collect()
    }
}

// A digest of the word counts that does not depend on the map's order, so the many words need no sorting: each word
// and its count hash on their own, and the hashes add up.
fn string_words_key(words: &FxHashMap<String, u32>) -> (usize, u64) {
    let sum = words.par_iter().map(|entry| {
        let mut hasher = CacheKeyHasher::new();
        entry.cache_key(&mut hasher);
        hasher.finish()
    });
    (words.len(), sum.reduce(|| 0, u64::wrapping_add))
}
