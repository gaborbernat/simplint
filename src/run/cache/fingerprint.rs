use std::hash::{Hash, Hasher};
use std::sync::Mutex;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::settings::LinterSettings;
use rustc_hash::FxHashMap;

use crate::lint::Linter;
use crate::lint::registry::RuleSet;
use crate::run::Mode;
use crate::settings::FileSettings;

#[derive(Clone, Copy)]
pub(crate) struct FileIdentity<'a> {
    pub(crate) settings: &'a FileSettings<'a>,
    pub(crate) ruff: &'a LinterSettings,
    pub(crate) module_path: Option<&'a Vec<String>>,
    pub(crate) sibling_init: bool,
}

impl<'a> FileIdentity<'a> {
    fn of(linter: &'a Linter<'a>) -> Self {
        Self {
            settings: &linter.settings,
            ruff: linter.ruff.settings,
            module_path: linter.module_path.as_ref(),
            sibling_init: linter.sibling_init,
        }
    }
}

// Most files share their project's settings and Ruff's, so those, Ruff's linter settings above all, are hashed once per
// pair.
#[derive(Default)]
pub(crate) struct Fingerprints(Mutex<FxHashMap<(usize, usize), u64>>);

impl Fingerprints {
    // The run's digest covers the index, so `base` with it says whether a stored result is still valid.
    pub(crate) fn base(&self, identity: &FileIdentity) -> u64 {
        let FileIdentity { settings, ruff, module_path, sibling_init } = *identity;
        let shared = *self
            .0
            .lock()
            .expect("no worker panics while fingerprinting")
            .entry((std::ptr::from_ref(settings.project).addr(), std::ptr::from_ref(ruff).addr()))
            .or_insert_with(|| {
                let mut hasher = CacheKeyHasher::new();
                settings.project.cache_key(&mut hasher);
                ruff.cache_key(&mut hasher);
                hasher.finish()
            });
        let mut hasher = CacheKeyHasher::new();
        shared.cache_key(&mut hasher);
        let rules = settings.rules;
        rules.cache_key(&mut hasher);
        settings.from_ruff.cache_key(&mut hasher);
        // An `__init__.py` added or removed next to the file moves its package root, which keeps the cache file. In a
        // namespace package that root stays, so the rules that read the file's `__init__.py` itself key it too.
        module_path.cache_key(&mut hasher);
        if rules.overlaps(RuleSet::sibling_init()) {
            sibling_init.cache_key(&mut hasher);
        }
        // A message naming a path relative to the working directory differs from one run elsewhere.
        if rules.overlaps(RuleSet::working_directory()) {
            std::env::current_dir().unwrap_or_default().cache_key(&mut hasher);
        }
        hasher.finish()
    }

    pub(crate) fn of(&self, linter: &Linter, mode: &Mode) -> Fingerprint {
        let base = self.base(&FileIdentity::of(linter));
        // Only a rule that reads other files depends on the index of the run, so the others keep their results when
        // another file changes.
        let clean = if linter.settings.rules.overlaps(RuleSet::cross_file()) {
            let mut hasher = CacheKeyHasher::new();
            base.cache_key(&mut hasher);
            linter.project.cache_key(&mut hasher);
            hasher.finish()
        } else {
            base
        };
        // Which fixes apply decides what the fix loop withdraws, and whether it changes the file at all.
        let settled = (!mode.fix.is_generate()).then(|| {
            let mut hasher = CacheKeyHasher::new();
            clean.cache_key(&mut hasher);
            mode.fix.hash(&mut hasher);
            mode.unsafe_fixes.cache_key(&mut hasher);
            mode.noqa.hash(&mut hasher);
            hasher.finish()
        });
        Fingerprint { clean, settled, base }
    }
}

#[derive(Clone, Copy)]
pub(crate) struct Fingerprint {
    pub(super) clean: u64,
    pub(super) settled: Option<u64>,
    pub(super) base: u64,
}
