use std::hash::Hasher;
use std::path::Path;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_db::diagnostic::{Diagnostic, DisplayDiagnosticConfig};
use ruff_diagnostics::Applicability;
use ruff_linter::message::{RenderContext, render_diagnostics};
use ruff_linter::settings::types::OutputFormat;
use rustc_hash::FxHashMap;

use crate::project::cache::read_aligned;
use crate::run::Diagnostics;

// The display settings a file's `full` text depends on, as the output chooses them.
#[derive(Clone, Copy)]
#[expect(clippy::struct_excessive_bools, reason = "each flag is one display setting the text depends on")]
pub(crate) struct DisplayKey {
    pub(crate) preview: bool,
    pub(crate) prefer_rule_codes: bool,
    pub(crate) color: bool,
    pub(crate) hyperlinks: bool,
    pub(crate) required: Applicability,
}

// The `full` settings a file's text is valid under: the display config and a digest of what else the text depends on.
// A file's text needs no run-wide state: the fix marker shows only on a diagnostic whose fix applies, and the run has
// such a fix only when one of its files does.
pub(crate) struct FullDisplay {
    config: DisplayDiagnosticConfig,
    base: u64,
}

impl FullDisplay {
    pub(crate) fn new(config: DisplayDiagnosticConfig, key: DisplayKey) -> Option<Self> {
        let cwd = std::env::current_dir().ok()?;
        let mut hasher = CacheKeyHasher::new();
        key.preview.cache_key(&mut hasher);
        key.prefer_rule_codes.cache_key(&mut hasher);
        key.color.cache_key(&mut hasher);
        key.hyperlinks.cache_key(&mut hasher);
        format!("{:?}", key.required).cache_key(&mut hasher);
        cwd.to_string_lossy().cache_key(&mut hasher);
        Some(Self { config, base: hasher.finish() })
    }

    // The key a file's text is stored under, given the name the output shows the file by.
    pub(super) fn key(&self, name: &str) -> u64 {
        let mut hasher = CacheKeyHasher::new();
        self.base.cache_key(&mut hasher);
        name.cache_key(&mut hasher);
        hasher.finish()
    }

    // The `full` text of one file's diagnostics, in the order the report sorts them.
    fn render(&self, diagnostics: &[Diagnostic]) -> Vec<u8> {
        let mut sorted = diagnostics.to_vec();
        sorted.sort_by(Diagnostic::ruff_start_ordering);
        let fixable = sorted.iter().any(|diagnostic| diagnostic.has_applicable_fix(self.config.fix_applicability()));
        let config = self.config.clone().with_show_fix_status(fixable);
        let notebooks = FxHashMap::default();
        let context = RenderContext::new(&notebooks);
        let mut buffer = Vec::new();
        render_diagnostics(&mut buffer, OutputFormat::Full, &config, &context, &sorted)
            .expect("writing to a vector cannot fail");
        buffer
    }
}

// A package's stored `full` text, which a missing or unreadable file leaves empty.
pub(super) fn read_rendered(path: &Path) -> FxHashMap<String, Rendered> {
    read_aligned(path)
        .ok()
        .and_then(|bytes| rkyv::from_bytes::<RenderedPackage, rkyv::rancor::Error>(&bytes).ok())
        .map(|package| package.files)
        .unwrap_or_default()
}

// The `full` text of a file's reports. It is valid for the file with `file` as its key, for the reports under
// `findings`, and for the display settings whose key is `key`.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct Rendered {
    pub(super) file: u64,
    pub(super) findings: u64,
    pub(super) key: u64,
    pub(super) text: Vec<u8>,
}

// A package's `full` text, by the path of each file relative to the package root, kept beside its cache file. Runs that
// show other output never read it.
#[derive(rkyv::Archive, rkyv::Serialize, rkyv::Deserialize)]
pub(super) struct RenderedPackage {
    pub(super) files: FxHashMap<String, Rendered>,
}

// The `full` text of a file's diagnostics, which the diagnostics carry to the printer.
pub(super) fn text(
    full: &FullDisplay,
    name: &str,
    diagnostics: &mut Diagnostics,
    file: u64,
    findings: u64,
) -> Rendered {
    let text = full.render(&diagnostics.inner);
    diagnostics.rendered.insert(name.to_owned(), text.clone());
    Rendered { file, findings, key: full.key(name), text }
}
