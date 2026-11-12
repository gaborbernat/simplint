use std::collections::BTreeSet;
use std::path::Path;

use globset::{Glob, GlobSet, GlobSetBuilder};
use ruff_cache::{CacheKey, CacheKeyHasher};

// The parts of the project's pytest configuration that rules read: the markers it registers and the file names it
// collects tests from. pytest reads the first of `pytest.ini`, `.pytest.ini`, `pyproject.toml`, `tox.ini` and
// `setup.cfg` that configures it, so the others never apply.
#[derive(Default)]
pub struct PytestConfig {
    // `None` when no file configures pytest, or the one that does cannot be read, so registration is unknown.
    pub(crate) markers: Option<BTreeSet<String>>,
    python_files: Vec<String>,
    collected: Option<GlobSet>,
}

impl PytestConfig {
    pub(super) fn of(directories: &[&Path], pyproject: Option<&toml::Table>) -> Self {
        directories.iter().find_map(|directory| read(directory, pyproject)).unwrap_or_else(|| Self::with(None, None))
    }

    fn with(markers: Option<BTreeSet<String>>, python_files: Option<Vec<String>>) -> Self {
        let python_files = python_files.unwrap_or_else(|| vec!["test_*.py".to_owned(), "*_test.py".to_owned()]);
        let mut builder = GlobSetBuilder::new();
        for pattern in &python_files {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        Self { markers, collected: builder.build().ok(), python_files }
    }

    // Whether pytest collects tests from a file of this name, as its `python_files` patterns match the base name.
    pub(crate) fn collects(&self, file_name: &str) -> bool {
        self.collected.as_ref().is_none_or(|set| set.is_match(file_name))
    }

    pub(crate) fn python_files(&self) -> &[String] {
        &self.python_files
    }
}

impl CacheKey for PytestConfig {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.markers.as_ref().map(|markers| markers.iter().collect::<Vec<_>>()).cache_key(state);
        self.python_files.cache_key(state);
    }
}

fn read(directory: &Path, pyproject: Option<&toml::Table>) -> Option<PytestConfig> {
    for name in ["pytest.ini", ".pytest.ini"] {
        let path = directory.join(name);
        if path.is_file() {
            // pytest takes this file even when it holds no `[pytest]` section.
            return Some(std::fs::read_to_string(path).map_or_else(
                |_| PytestConfig::with(None, None),
                |text| from_ini(&text, "pytest").unwrap_or_else(|| PytestConfig::with(Some(BTreeSet::new()), None)),
            ));
        }
    }
    if directory.join("pyproject.toml").is_file()
        && let Some(config) = pyproject.and_then(from_pyproject)
    {
        return Some(config);
    }
    [("tox.ini", "pytest"), ("setup.cfg", "tool:pytest")].iter().find_map(|(name, section)| {
        let text = std::fs::read_to_string(directory.join(name)).ok()?;
        from_ini(&text, section)
    })
}

fn from_pyproject(pyproject: &toml::Table) -> Option<PytestConfig> {
    let pytest = pyproject.get("tool")?.get("pytest")?;
    // pytest 9 reads its options from `[tool.pytest]` itself; earlier versions from its `ini_options` table.
    let options = pytest.get("ini_options").unwrap_or(pytest);
    let strings = |key: &str| -> Option<Vec<String>> {
        match options.get(key)? {
            toml::Value::String(text) => Some(text.split_whitespace().map(str::to_owned).collect()),
            toml::Value::Array(items) => Some(items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()),
            _ => None,
        }
    };
    let markers = match options.get("markers") {
        None => Some(BTreeSet::new()),
        Some(toml::Value::Array(items)) => {
            items.iter().map(|item| item.as_str().map(marker_name)).collect::<Option<BTreeSet<_>>>()
        }
        Some(toml::Value::String(text)) => Some(text.lines().map(marker_name).filter(|name| !name.is_empty()).collect()),
        Some(_) => None,
    };
    Some(PytestConfig::with(markers, strings("python_files")))
}

// An INI file's section as pytest reads it, `None` when the file has no such section. A value continues on the
// indented lines below its key.
fn from_ini(text: &str, wanted: &str) -> Option<PytestConfig> {
    let mut in_section = false;
    let mut found = false;
    let mut values: Vec<(String, String)> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with(['#', ';']) {
            continue;
        }
        if let Some(section) = trimmed.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            in_section = section.trim() == wanted;
            found |= in_section;
            continue;
        }
        if !in_section {
            continue;
        }
        if line.starts_with([' ', '\t']) {
            if let Some((_, value)) = values.last_mut() {
                value.push('\n');
                value.push_str(trimmed);
            }
        } else if let Some((key, value)) = trimmed.split_once(['=', ':']) {
            values.push((key.trim().to_owned(), value.trim().to_owned()));
        }
    }
    if !found {
        return None;
    }
    let value = |key: &str| values.iter().find(|(name, _)| name == key).map(|(_, value)| value.as_str());
    let markers = value("markers")
        .map_or_else(BTreeSet::new, |text| text.lines().map(marker_name).filter(|name| !name.is_empty()).collect());
    let python_files = value("python_files").map(|text| text.split_whitespace().map(str::to_owned).collect());
    Some(PytestConfig::with(Some(markers), python_files))
}

// A marker line names the marker before its arguments or description: `slow: marks slow tests`, `env(name): ...`.
fn marker_name(line: &str) -> String {
    line.split([':', '(']).next().unwrap_or_default().trim().to_owned()
}
