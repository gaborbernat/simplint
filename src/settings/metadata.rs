use std::collections::{BTreeMap, BTreeSet};

use serde::Deserialize;

// The `[project]` table of a `pyproject.toml`, reduced to what simplint's rules read: the project's name, the modules
// its unconditional requirements install, and the objects its scripts and entry points name.
#[derive(Deserialize, Default)]
#[serde(from = "ProjectTable")]
pub struct ProjectMetadata {
    pub name: Option<String>,
    pub dependencies: BTreeSet<String>,
    pub entry_points: Vec<String>,
}

impl From<ProjectTable> for ProjectMetadata {
    fn from(project: ProjectTable) -> Self {
        Self {
            name: project.name,
            // A requirement with an environment marker is conditional, so a guarded import of it is legitimate.
            dependencies: project
                .dependencies
                .iter()
                .filter(|spec| !spec.contains(';'))
                .flat_map(|spec| import_names(spec))
                .collect(),
            entry_points: project
                .scripts
                .into_values()
                .chain(project.gui_scripts.into_values())
                .chain(project.entry_points.into_values().flat_map(BTreeMap::into_values))
                .collect(),
        }
    }
}

pub(super) fn import_names(requirement: &str) -> Vec<String> {
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

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "kebab-case")]
struct ProjectTable {
    name: Option<String>,
    dependencies: Vec<String>,
    scripts: BTreeMap<String, String>,
    gui_scripts: BTreeMap<String, String>,
    entry_points: BTreeMap<String, BTreeMap<String, String>>,
}

// How a `pyproject.toml` builds the project: whether a `[build-system]` declares a backend, the packages the backend's
// own settings ship, and the settings that make a build install no Python package.
#[derive(Default)]
pub struct BuildConfiguration {
    pub has_build_system: bool,
    pub packages: Vec<String>,
    pub uv_package: Option<bool>,
    pub maturin_bindings: Option<String>,
}

impl BuildConfiguration {
    pub(super) fn of(pyproject: &toml::Table) -> Self {
        let lookup = |path: &[&str]| path.iter().try_fold(pyproject.get("tool")?, |value, key| value.get(key));
        let strings = |value: Option<&toml::Value>| -> Vec<String> {
            match value {
                Some(toml::Value::String(text)) => vec![text.clone()],
                Some(toml::Value::Array(items)) => {
                    items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect()
                }
                _ => Vec::new(),
            }
        };
        // Each backend names the importable packages it ships its own way; a `find` table discovers them instead.
        let packages = [
            &["hatch", "build", "targets", "wheel", "packages"][..],
            &["hatch", "build", "packages"],
            &["uv", "build-backend", "module-name"],
            &["flit", "module", "name"],
            &["setuptools", "packages"],
            &["setuptools", "py-modules"],
            &["maturin", "module-name"],
        ]
        .iter()
        .flat_map(|path| strings(lookup(path)))
        .collect();
        Self {
            has_build_system: pyproject.contains_key("build-system"),
            packages,
            uv_package: lookup(&["uv", "package"]).and_then(toml::Value::as_bool),
            maturin_bindings: lookup(&["maturin", "bindings"]).and_then(toml::Value::as_str).map(str::to_owned),
        }
    }
}
