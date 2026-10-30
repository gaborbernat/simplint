use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use globset::{Glob, GlobMatcher, GlobSet, GlobSetBuilder};
use ruff_python_ast::PythonVersion;
use serde::Deserialize;

use crate::registry::RuleSet;

pub struct Settings {
    pub root: PathBuf,
    pub target_version: PythonVersion,
    pub dependencies: HashSet<String>,
    pub entry_points: Vec<String>,
    rules: RuleSet,
    per_file_ignores: Vec<(GlobMatcher, RuleSet)>,
    exclude: GlobSet,
}

impl Settings {
    pub fn discover(path: &Path) -> Result<Self> {
        let absolute = std::path::absolute(path)?;
        let start = if absolute.is_dir() { absolute.as_path() } else { absolute.parent().unwrap_or(&absolute) };
        match start.ancestors().find(|dir| dir.join("pyproject.toml").is_file()) {
            Some(root) => Self::from_pyproject(root),
            None => Ok(Self::new(start.to_path_buf())),
        }
    }

    fn from_pyproject(root: &Path) -> Result<Self> {
        let path = root.join("pyproject.toml");
        let text = std::fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        let pyproject: Pyproject = toml::from_str(&text).with_context(|| format!("parsing {}", path.display()))?;
        let ruff = pyproject.tool.ruff;
        let simplint = pyproject.tool.simplint;
        let mut settings = Self::new(root.to_path_buf());
        if let Some(version) = ruff
            .target_version
            .as_deref()
            .and_then(Self::parse_ruff_target)
            .or_else(|| pyproject.project.requires_python.as_deref().and_then(Self::parse_requires_python))
        {
            settings.target_version = version;
        }
        // A requirement with an environment marker is conditional, so a guarded import of it is legitimate.
        settings.dependencies = pyproject
            .project
            .dependencies
            .iter()
            .filter(|spec| !spec.contains(';'))
            .flat_map(|spec| Self::import_names(spec))
            .collect();
        settings.entry_points = pyproject
            .project
            .scripts
            .into_values()
            .chain(pyproject.project.gui_scripts.into_values())
            .chain(pyproject.project.entry_points.into_values().flat_map(BTreeMap::into_values))
            .collect();
        settings.rules = RuleSet::from_selectors(&simplint.select.unwrap_or_else(|| vec!["ALL".to_owned()]))?
            .union(RuleSet::from_selectors(&simplint.extend_select)?)
            .subtract(RuleSet::from_selectors(&simplint.ignore)?);
        settings.per_file_ignores = simplint
            .per_file_ignores
            .iter()
            .map(|(pattern, selectors)| {
                Ok((Glob::new(pattern)?.compile_matcher(), RuleSet::from_selectors(selectors)?))
            })
            .collect::<Result<_>>()?;
        let mut exclude = GlobSetBuilder::new();
        for pattern in ruff.exclude.iter().chain(&ruff.extend_exclude) {
            exclude.add(Glob::new(pattern)?);
        }
        settings.exclude = exclude.build()?;
        Ok(settings)
    }

    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            target_version: PythonVersion::PY310,
            dependencies: HashSet::new(),
            entry_points: Vec::new(),
            rules: RuleSet::all(),
            per_file_ignores: Vec::new(),
            exclude: GlobSet::empty(),
        }
    }

    fn parse_ruff_target(target: &str) -> Option<PythonVersion> {
        let (major, minor) = target.strip_prefix("py")?.split_at(1);
        Some(PythonVersion { major: major.parse().ok()?, minor: minor.parse().ok()? })
    }

    fn parse_requires_python(specifier: &str) -> Option<PythonVersion> {
        let lower = specifier.split(',').find_map(|part| part.trim().strip_prefix(">="))?;
        let mut parts = lower.trim().split('.');
        Some(PythonVersion { major: parts.next()?.parse().ok()?, minor: parts.next()?.parse().ok()? })
    }

    fn import_names(requirement: &str) -> Vec<String> {
        // Distributions whose import name differs from the project name; the rest import under their normalized name.
        const RENAMED: &[(&str, &str)] = &[
            ("attrs", "attr"),
            ("beautifulsoup4", "bs4"),
            ("opencv_python", "cv2"),
            ("opencv_python_headless", "cv2"),
            ("pillow", "pil"),
            ("protobuf", "google"),
            ("pyjwt", "jwt"),
            ("python_dateutil", "dateutil"),
            ("python_dotenv", "dotenv"),
            ("pyyaml", "yaml"),
            ("pyzmq", "zmq"),
            ("scikit_learn", "sklearn"),
            ("setuptools", "pkg_resources"),
        ];
        let name: String = requirement
            .chars()
            .take_while(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
            .map(|char| if matches!(char, '-' | '.') { '_' } else { char.to_ascii_lowercase() })
            .collect();
        let renamed = RENAMED.iter().filter(|(project, _)| *project == name).map(|(_, import)| (*import).to_owned());
        renamed.chain([name.clone()]).collect()
    }

    pub fn with_rules(mut self, select: &[String], ignore: &[String]) -> Result<Self> {
        if !select.is_empty() {
            self.rules = RuleSet::from_selectors(select)?;
        }
        self.rules = self.rules.subtract(RuleSet::from_selectors(ignore)?);
        Ok(self)
    }

    pub fn is_excluded(&self, path: &Path) -> bool {
        self.relative(path)
            .ancestors()
            .any(|ancestor| !ancestor.as_os_str().is_empty() && self.exclude.is_match(ancestor))
    }

    fn relative(&self, path: &Path) -> PathBuf {
        let absolute = std::path::absolute(path).unwrap_or_default();
        absolute.strip_prefix(&self.root).map_or_else(|_| absolute.clone(), Path::to_path_buf)
    }

    pub fn rules_for(&self, path: &Path) -> RuleSet {
        let relative = self.relative(path);
        self.per_file_ignores
            .iter()
            .filter(|(matcher, _)| {
                matcher.is_match(&relative) || relative.file_name().is_some_and(|name| matcher.is_match(name))
            })
            .fold(self.rules, |rules, (_, ignored)| rules.subtract(*ignored))
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Pyproject {
    project: Project,
    tool: Tool,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct Project {
    requires_python: Option<String>,
    dependencies: Vec<String>,
    scripts: BTreeMap<String, String>,
    gui_scripts: BTreeMap<String, String>,
    entry_points: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct Tool {
    ruff: RuffOptions,
    simplint: SimplintOptions,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct RuffOptions {
    target_version: Option<String>,
    exclude: Vec<String>,
    extend_exclude: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct SimplintOptions {
    select: Option<Vec<String>>,
    extend_select: Vec<String>,
    ignore: Vec<String>,
    per_file_ignores: BTreeMap<String, Vec<String>>,
}
