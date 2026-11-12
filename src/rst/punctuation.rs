// The punctuation classes of docutils' inline markup rules, which decide where a start-string or an end-string may sit.
use std::collections::HashSet;
use std::sync::LazyLock;

// Copied from `docutils/utils/punctuation_chars.py` of docutils 0.23, which generates them from Unicode character
// categories, so they match docutils' recognition exactly. Regenerate them from that file rather than editing them.

// The strings are regular expression character classes, as docutils embeds them, so `match_chars` reads them by position.
pub(crate) const OPENERS: &str = concat!(
    "\u{22}'(<\u{5c}[{\u{f3a}\u{f3c}\u{169b}\u{2045}\u{207d}\u{208d}\u{2329}\u{2768}\u{276a}\u{276c}\u{276e}\u{2770}\u{2772}\u{2774}\u{27c5}\u{27e6}\u{27e8}",
    "\u{27ea}\u{27ec}\u{27ee}\u{2983}\u{2985}\u{2987}\u{2989}\u{298b}\u{298d}\u{298f}\u{2991}\u{2993}\u{2995}\u{2997}\u{29d8}\u{29da}\u{29fc}\u{2e22}\u{2e24}\u{2e26}\u{2e28}\u{3008}\u{300a}\u{300c}",
    "\u{300e}\u{3010}\u{3014}\u{3016}\u{3018}\u{301a}\u{301d}\u{301d}\u{fd3e}\u{fe17}\u{fe35}\u{fe37}\u{fe39}\u{fe3b}\u{fe3d}\u{fe3f}\u{fe41}\u{fe43}\u{fe47}\u{fe59}\u{fe5b}\u{fe5d}\u{ff08}\u{ff3b}",
    "\u{ff5b}\u{ff5f}\u{ff62}\u{ab}\u{2018}\u{201c}\u{2039}\u{2e02}\u{2e04}\u{2e09}\u{2e0c}\u{2e1c}\u{2e20}\u{201a}\u{201e}\u{bb}\u{2019}\u{201d}\u{203a}\u{2e03}\u{2e05}\u{2e0a}\u{2e0d}\u{2e1d}",
    "\u{2e21}\u{201b}\u{201f}",
);

pub(crate) const CLOSERS: &str = concat!(
    "\u{22}')>\u{5c}]}\u{f3b}\u{f3d}\u{169c}\u{2046}\u{207e}\u{208e}\u{232a}\u{2769}\u{276b}\u{276d}\u{276f}\u{2771}\u{2773}\u{2775}\u{27c6}\u{27e7}\u{27e9}",
    "\u{27eb}\u{27ed}\u{27ef}\u{2984}\u{2986}\u{2988}\u{298a}\u{298c}\u{298e}\u{2990}\u{2992}\u{2994}\u{2996}\u{2998}\u{29d9}\u{29db}\u{29fd}\u{2e23}\u{2e25}\u{2e27}\u{2e29}\u{3009}\u{300b}\u{300d}",
    "\u{300f}\u{3011}\u{3015}\u{3017}\u{3019}\u{301b}\u{301e}\u{301f}\u{fd3f}\u{fe18}\u{fe36}\u{fe38}\u{fe3a}\u{fe3c}\u{fe3e}\u{fe40}\u{fe42}\u{fe44}\u{fe48}\u{fe5a}\u{fe5c}\u{fe5e}\u{ff09}\u{ff3d}",
    "\u{ff5d}\u{ff60}\u{ff63}\u{bb}\u{2019}\u{201d}\u{203a}\u{2e03}\u{2e05}\u{2e0a}\u{2e0d}\u{2e1d}\u{2e21}\u{201b}\u{201f}\u{ab}\u{2018}\u{201c}\u{2039}\u{2e02}\u{2e04}\u{2e09}\u{2e0c}\u{2e1c}",
    "\u{2e20}\u{201a}\u{201e}",
);

pub(crate) const DELIMITERS: &str = concat!(
    "\u{5c}-/:\u{58a}\u{a1}\u{b7}\u{bf}\u{37e}\u{387}\u{55a}-\u{55f}\u{589}\u{5be}\u{5c0}\u{5c3}\u{5c6}\u{5f3}\u{5f4}\u{609}\u{60a}\u{60c}\u{60d}",
    "\u{61b}\u{61e}\u{61f}\u{66a}-\u{66d}\u{6d4}\u{700}-\u{70d}\u{7f7}-\u{7f9}\u{830}-\u{83e}\u{964}\u{965}\u{970}\u{df4}\u{e4f}\u{e5a}\u{e5b}\u{f04}",
    "-\u{f12}\u{f85}\u{fd0}-\u{fd4}\u{104a}-\u{104f}\u{10fb}\u{1361}-\u{1368}\u{1400}\u{166d}\u{166e}\u{16eb}-\u{16ed}\u{1735}\u{1736}\u{17d4}-\u{17d6}",
    "\u{17d8}-\u{17da}\u{1800}-\u{180a}\u{1944}\u{1945}\u{19de}\u{19df}\u{1a1e}\u{1a1f}\u{1aa0}-\u{1aa6}\u{1aa8}-\u{1aad}\u{1b5a}-\u{1b60}\u{1c3b}-\u{1c3f}",
    "\u{1c7e}\u{1c7f}\u{1cd3}\u{2010}-\u{2017}\u{2020}-\u{2027}\u{2030}-\u{2038}\u{203b}-\u{203e}\u{2041}-\u{2043}\u{2047}-\u{2051}\u{2053}\u{2055}-",
    "\u{205e}\u{2cf9}-\u{2cfc}\u{2cfe}\u{2cff}\u{2e00}\u{2e01}\u{2e06}-\u{2e08}\u{2e0b}\u{2e0e}-\u{2e1b}\u{2e1e}\u{2e1f}\u{2e2a}-\u{2e2e}\u{2e30}\u{2e31}\u{3001}-",
    "\u{3003}\u{301c}\u{3030}\u{303d}\u{30a0}\u{30fb}\u{a4fe}\u{a4ff}\u{a60d}-\u{a60f}\u{a673}\u{a67e}\u{a6f2}-\u{a6f7}\u{a874}-\u{a877}\u{a8ce}\u{a8cf}\u{a8f8}-\u{a8fa}",
    "\u{a92e}\u{a92f}\u{a95f}\u{a9c1}-\u{a9cd}\u{a9de}\u{a9df}\u{aa5c}-\u{aa5f}\u{aade}\u{aadf}\u{abeb}\u{fe10}-\u{fe16}\u{fe19}\u{fe30}-\u{fe32}\u{fe45}\u{fe46}\u{fe49}",
    "-\u{fe4c}\u{fe50}-\u{fe52}\u{fe54}-\u{fe58}\u{fe5f}-\u{fe61}\u{fe63}\u{fe68}\u{fe6a}\u{fe6b}\u{ff01}-\u{ff03}\u{ff05}-\u{ff07}\u{ff0a}\u{ff0c}-",
    "\u{ff0f}\u{ff1a}\u{ff1b}\u{ff1f}\u{ff20}\u{ff3c}\u{ff61}\u{ff64}\u{ff65}\u{10100}\u{10101}\u{1039f}\u{103d0}\u{10857}\u{1091f}\u{1093f}\u{10a50}-\u{10a58}\u{10a7f}\u{10b39}-\u{10b3f}\u{110bb}",
    "\u{110bc}\u{110be}-\u{110c1}\u{12470}-\u{12473}",
);

pub(crate) const CLOSING_DELIMITERS: &str = "\u{5c}\u{5c}.,;!?";

// The pairs of quotation marks that close each other beyond the one at the same index of `OPENERS` and `CLOSERS`.
pub(crate) const QUOTE_PAIRS: [(char, &str); 8] = [
    ('\u{bb}', "\u{bb}"),
    ('\u{2018}', "\u{201a}"),
    ('\u{2019}', "\u{2019}"),
    ('\u{201a}', "\u{2018}\u{2019}"),
    ('\u{201c}', "\u{201e}"),
    ('\u{201e}', "\u{201c}\u{201d}"),
    ('\u{201d}', "\u{201d}"),
    ('\u{203a}', "\u{203a}"),
];

// Whether `closer` closes `opener` under docutils' `match_chars`: the closer at the same index, or a quote pair.
pub(crate) fn match_chars(opener: char, closer: char) -> bool {
    OPENERS.chars().position(|char| char == opener).is_some_and(|at| CLOSERS.chars().nth(at) == Some(closer))
        || QUOTE_PAIRS.iter().any(|(open, closers)| *open == opener && closers.contains(closer))
}

pub(crate) static OPENER_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| members(OPENERS));
pub(crate) static CLOSER_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| members(CLOSERS));
pub(crate) static DELIMITER_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| members(DELIMITERS));
pub(crate) static CLOSING_DELIMITER_CHARS: LazyLock<HashSet<char>> = LazyLock::new(|| members(CLOSING_DELIMITERS));

// The characters a regular expression character class matches: `\x` stands for `x`, and `a-b` for the characters from
// `a` through `b`, as docutils' classes use both.
fn members(class: &str) -> HashSet<char> {
    let mut items: Vec<(char, bool)> = Vec::new();
    let mut chars = class.chars();
    while let Some(char) = chars.next() {
        if char == '\\' {
            items.extend(chars.next().map(|escaped| (escaped, true)));
        } else {
            items.push((char, false));
        }
    }
    let mut set = HashSet::new();
    let mut at = 0;
    while let Some(&(start, _)) = items.get(at) {
        if let Some([('-', false), (end, _)]) = items.get(at + 1..at + 3) {
            set.extend(start..=*end);
            at += 3;
        } else {
            set.insert(start);
            at += 1;
        }
    }
    set
}
