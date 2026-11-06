use rustc_hash::FxHashSet;

// The words a file's string literals hold, sorted and one per line, which the cache reads back in one allocation.
pub(super) fn string_words(words: FxHashSet<String>) -> String {
    let mut words: Vec<String> = words.into_iter().collect();
    words.sort_unstable();
    words.join("\n")
}
