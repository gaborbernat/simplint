use std::hash::Hasher;

use rayon::prelude::*;
use rustc_hash::{FxHashMap, FxHashSet};

use ruff_cache::{CacheKey, CacheKeyHasher};

use crate::project::computed_reads::ComputedReads;
use crate::project::facts::Channel;
use crate::project::{DocReferences, FileFacts};

use super::files::FileId;
use super::writes::Writes;
use super::{every, every_in};
use crate::syntax::last_dotted_segment;

pub(in crate::project) struct RunWide {
    pub(super) reads: RunReads,
    pub(super) assigned_attributes: FxHashSet<String>,
    pub(super) module_reads: FxHashSet<String>,
    pub(super) base_classes: FxHashSet<String>,
    pub(super) fixture_requests: FxHashSet<String>,
    // Names the index cannot follow: those read from a module that resolves to no file of the run, and doctest uses.
    pub(super) opaque_names: FxHashSet<String>,
    // The dotted names a string or store of the run reaches, such as `requests.get` in `mock.patch("requests.get")`,
    // whether or not their module is a file of the run.
    pub(super) referenced_paths: FxHashSet<String>,
    pub(super) computed_reads: ComputedReads,
    pub(super) writes: Writes,
    // The class names more than one module-level class of the run defines, which a documentation reference by simple
    // name cannot tell apart.
    pub(super) class_namesakes: FxHashSet<String>,
    pub(super) bound_marks: FxHashSet<String>,
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
    let referenced_paths = indexed
        .iter()
        .flat_map(|(_, facts)| &facts.names[Channel::Reference])
        .flat_map(|(module, names)| names.iter().map(move |name| format!("{module}.{name}")))
        .collect();
    RunWide {
        reads,
        assigned_attributes,
        module_reads,
        base_classes,
        fixture_requests,
        opaque_names,
        referenced_paths,
        computed_reads,
        writes,
        class_namesakes: class_namesakes(indexed),
        bound_marks: every(indexed, |facts| &facts.bound_marks),
    }
}

fn class_namesakes(files: &[(FileId, &FileFacts)]) -> FxHashSet<String> {
    let mut seen = FxHashSet::default();
    let mut shared = FxHashSet::default();
    for class in files.iter().flat_map(|(_, facts)| &facts.classes) {
        if !seen.insert(class.name.as_str()) {
            shared.insert(class.name.clone());
        }
    }
    shared
}

fn fixture_requests(files: &[(FileId, &FileFacts)], test_modules: &FxHashSet<FileId>) -> FxHashSet<String> {
    let mut requests = every_in(files, |id| test_modules.contains(&id), |facts| &facts.fixture_requests);
    requests.extend(every(files, |facts| &facts.pytest_requests));
    requests
}

fn computed_reads(files: &[(FileId, &FileFacts)]) -> ComputedReads {
    let mut reads = ComputedReads::default();
    for (_, facts) in files {
        reads.extend(&facts.computed_reads);
    }
    reads
}

#[derive(Default, PartialEq)]
pub(super) struct RunReads {
    pub(super) docstrings: bool,
    pub(super) class_names: bool,
    pub(super) reflected: FxHashSet<String>,
    pub(super) string_words: FxHashMap<String, u32>,
    // Whether a test reads `__all__`, by its name in a string as `getattr(module, "__all__")` does or as an attribute,
    // which may check what type it is.
    pub(super) tests_read_all: bool,
}

impl RunReads {
    pub(super) fn collect(indexed: &[(FileId, &FileFacts)], test_modules: &FxHashSet<FileId>) -> Self {
        Self {
            docstrings: indexed.iter().any(|(_, facts)| facts.reads_docstrings),
            class_names: indexed.iter().any(|(_, facts)| facts.reads_class_names),
            reflected: indexed.iter().flat_map(|(_, facts)| facts.reflected_names.iter().cloned()).collect(),
            string_words: string_word_counts(indexed),
            tests_read_all: indexed.iter().any(|(id, facts)| {
                test_modules.contains(id)
                    && (facts.string_words.lines().any(|word| word == "__all__")
                        || facts.attribute_loads.contains_key("__all__"))
            }),
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
    pub(super) fn digest(&self, docs: &DocReferences) -> u64 {
        let mut hasher = CacheKeyHasher::new();
        self.reads.docstrings.cache_key(&mut hasher);
        self.reads.class_names.cache_key(&mut hasher);
        self.reads.reflected.cache_key(&mut hasher);
        docs.cache_key(&mut hasher);
        self.computed_reads.cache_key(&mut hasher);
        string_words_key(&self.reads.string_words).cache_key(&mut hasher);
        self.reads.tests_read_all.cache_key(&mut hasher);
        // Each set is hashed behind its length, so a name moving from one set to the next changes the digest.
        for names in [
            &self.assigned_attributes,
            &self.module_reads,
            &self.base_classes,
            &self.fixture_requests,
            &self.opaque_names,
            &self.referenced_paths,
            &self.bound_marks,
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

    // Low protocol pickling concerns every dataclass, so its change names `dataclass`.
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
            .map(|class| last_dotted_segment(class));
        let pickling = (writes.pickles_low_protocol != others.pickles_low_protocol).then_some("dataclass");
        let all_read = (self.reads.tests_read_all != other.reads.tests_read_all).then_some("__all__");
        self.assigned_attributes
            .symmetric_difference(&other.assigned_attributes)
            .chain(self.module_reads.symmetric_difference(&other.module_reads))
            .chain(self.base_classes.symmetric_difference(&other.base_classes))
            .chain(self.fixture_requests.symmetric_difference(&other.fixture_requests))
            .chain(self.opaque_names.symmetric_difference(&other.opaque_names))
            .chain(self.bound_marks.symmetric_difference(&other.bound_marks))
            .chain(writes.untyped.symmetric_difference(&others.untyped))
            .map(String::as_str)
            .chain(
                self.referenced_paths
                    .symmetric_difference(&other.referenced_paths)
                    .map(|path| last_dotted_segment(path)),
            )
            .chain(classes)
            .chain(pickling)
            .chain(all_read)
            .chain(self.string_word_changes(other))
            .collect()
    }

    fn string_word_changes<'p>(&'p self, other: &'p Self) -> impl Iterator<Item = &'p str> {
        let (own, others) = (&self.reads.string_words, &other.reads.string_words);
        let counts =
            own.iter().filter(|(word, count)| others.get(*word) != Some(*count)).map(|(word, _)| word.as_str());
        let only_other = others.keys().filter(|word| !own.contains_key(*word));
        counts.chain(only_other.map(String::as_str))
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
