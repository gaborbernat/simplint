use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use globset::{Glob, GlobSet, GlobSetBuilder};
use ruff_cache::{CacheKey, CacheKeyHasher};

// The parts of the project's pytest configuration that simplint reads: the markers it registers, the files it collects
// tests from, the plugins `addopts` loads and the directories `pythonpath` adds to the import path. pytest reads the
// first of `pytest.ini`, `.pytest.ini`, `pyproject.toml`, `tox.ini` and `setup.cfg` that configures it, so the others
// never apply.
#[derive(Default)]
pub struct PytestConfig {
    // `None` when no file configures pytest, or the one that does cannot be read, so registration is unknown.
    pub(crate) markers: Option<BTreeSet<String>>,
    python_files: Vec<String>,
    collected: Option<GlobSet>,
    pub(crate) plugins: Vec<String>,
    pub(crate) pythonpath: Vec<PathBuf>,
    testpaths: Vec<PathBuf>,
}

// The options as a configuration file spells them, before the defaults fill in what it leaves out.
#[derive(Default)]
struct Options {
    markers: Option<BTreeSet<String>>,
    python_files: Option<Vec<String>>,
    addopts: Vec<String>,
    pythonpath: Vec<String>,
    testpaths: Vec<String>,
}

impl PytestConfig {
    pub(super) fn of(directories: &[&Path], pyproject: Option<&toml::Table>) -> Self {
        directories
            .iter()
            .find_map(|directory| read(directory, pyproject))
            .unwrap_or_else(|| Self::with(Path::new(""), Options::default()))
    }

    // `pythonpath` and `testpaths` are relative to the directory of the file that sets them.
    fn with(directory: &Path, options: Options) -> Self {
        let python_files = options.python_files.unwrap_or_else(|| vec!["test_*.py".to_owned(), "*_test.py".to_owned()]);
        let mut builder = GlobSetBuilder::new();
        for pattern in &python_files {
            if let Ok(glob) = Glob::new(pattern) {
                builder.add(glob);
            }
        }
        let plugins = options
            .addopts
            .windows(2)
            .filter(|pair| pair[0] == "-p")
            .map(|pair| pair[1].as_str())
            .chain(options.addopts.iter().filter_map(|option| option.strip_prefix("-p")))
            .filter(|plugin| !plugin.is_empty() && !plugin.starts_with("no:"))
            .map(str::to_owned)
            .collect();
        let relative = |paths: Vec<String>| paths.into_iter().map(|path| directory.join(path)).collect();
        Self {
            markers: options.markers,
            collected: builder.build().ok(),
            python_files,
            plugins,
            pythonpath: relative(options.pythonpath),
            testpaths: relative(options.testpaths),
        }
    }

    // Whether pytest collects tests from a file of this name, as its `python_files` patterns match the base name.
    pub(crate) fn collects(&self, file_name: &str) -> bool {
        self.collected.as_ref().is_none_or(|set| set.is_match(file_name))
    }

    // Whether a run of pytest without arguments collects tests from the file: its name matches `python_files`, under
    // one of the `testpaths` when the configuration sets them.
    pub(crate) fn collects_path(&self, path: &Path) -> bool {
        path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
            self.collected.as_ref().is_some_and(|set| set.is_match(name))
        }) && (self.testpaths.is_empty() || self.testpaths.iter().any(|directory| path.starts_with(directory)))
    }

    pub(crate) fn python_files(&self) -> &[String] {
        &self.python_files
    }
}

impl CacheKey for PytestConfig {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.markers.as_ref().map(|markers| markers.iter().collect::<Vec<_>>()).cache_key(state);
        self.python_files.cache_key(state);
        self.plugins.cache_key(state);
        self.pythonpath.cache_key(state);
        self.testpaths.cache_key(state);
    }
}

fn read(directory: &Path, pyproject: Option<&toml::Table>) -> Option<PytestConfig> {
    for name in ["pytest.ini", ".pytest.ini"] {
        let path = directory.join(name);
        if path.is_file() {
            // pytest takes this file even when it holds no `[pytest]` section.
            let options = std::fs::read_to_string(path).map_or_else(
                |_| Options::default(),
                |text| {
                    from_ini(&text, "pytest").unwrap_or_else(|| Options { markers: Some(BTreeSet::new()), ..Options::default() })
                },
            );
            return Some(PytestConfig::with(directory, options));
        }
    }
    if directory.join("pyproject.toml").is_file()
        && let Some(options) = pyproject.and_then(from_pyproject)
    {
        return Some(PytestConfig::with(directory, options));
    }
    [("tox.ini", "pytest"), ("setup.cfg", "tool:pytest")].iter().find_map(|(name, section)| {
        let text = std::fs::read_to_string(directory.join(name)).ok()?;
        Some(PytestConfig::with(directory, from_ini(&text, section)?))
    })
}

fn from_pyproject(pyproject: &toml::Table) -> Option<Options> {
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
    Some(Options {
        markers,
        python_files: strings("python_files"),
        addopts: strings("addopts").unwrap_or_default(),
        pythonpath: strings("pythonpath").unwrap_or_default(),
        testpaths: strings("testpaths").unwrap_or_default(),
    })
}

// An INI file's section as pytest reads it, `None` when the file has no such section. A value continues on the
// indented lines below its key.
fn from_ini(text: &str, wanted: &str) -> Option<Options> {
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
    let words = |key: &str| value(key).map(|text| text.split_whitespace().map(str::to_owned).collect());
    Some(Options {
        markers: Some(markers),
        python_files: words("python_files"),
        addopts: words("addopts").unwrap_or_default(),
        pythonpath: words("pythonpath").unwrap_or_default(),
        testpaths: words("testpaths").unwrap_or_default(),
    })
}

// A marker line names the marker before its arguments or description: `slow: marks slow tests`, `env(name): ...`.
fn marker_name(line: &str) -> String {
    line.split([':', '(']).next().unwrap_or_default().trim().to_owned()
}
