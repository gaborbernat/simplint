use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

// The `[project]` table of a `pyproject.toml`, reduced to what simplint's rules read: the project's name, the modules
// its unconditional requirements install, and the objects its scripts and entry points name.
#[derive(Deserialize, Default)]
#[serde(from = "ProjectTable")]
pub struct ProjectMetadata {
    pub name: Option<String>,
    pub dependencies: BTreeSet<String>,
    // Every module the project lists, whatever its marker, in optional dependencies and dependency groups too. A rule
    // asks whether a tool is declared at all, which an install may leave out.
    pub declared: BTreeSet<String>,
    pub entry_points: Vec<String>,
}

impl ProjectMetadata {
    // `[dependency-groups]` sits outside `[project]`, so the parsed table cannot carry it. Its `include-group` tables
    // name no requirement and are skipped.
    pub(super) fn add_groups(&mut self, groups: &BTreeMap<String, Vec<toml::Value>>) {
        self.declared.extend(groups.values().flatten().filter_map(toml::Value::as_str).flat_map(import_names));
    }
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
            declared: project
                .dependencies
                .iter()
                .chain(project.optional_dependencies.values().flatten())
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
    optional_dependencies: BTreeMap<String, Vec<String>>,
    scripts: BTreeMap<String, String>,
    gui_scripts: BTreeMap<String, String>,
    entry_points: BTreeMap<String, BTreeMap<String, String>>,
}

// How a `pyproject.toml` builds the project: whether a `[build-system]` declares a backend, the packages the backend's
// own settings ship, the settings that make a build install no Python package, and the backend a build frontend loads.
#[derive(Default)]
pub struct BuildConfiguration {
    pub has_build_system: bool,
    pub packages: Vec<String>,
    pub uv_package: Option<bool>,
    pub maturin_bindings: Option<String>,
    // The `build-backend` as written, `module` or `module:object`, and the `backend-path` directories it loads from.
    pub backend: Option<String>,
    pub backend_path: Vec<String>,
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
        let system = pyproject.get("build-system").and_then(toml::Value::as_table);
        let system_value = |key: &str| system.and_then(|table| table.get(key));
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
            backend: system_value("build-backend").and_then(toml::Value::as_str).map(str::to_owned),
            backend_path: strings(system_value("backend-path")),
        }
    }
}

// The file and names of the build backend, each name paired with the file that defines it. A backend written as
// `module:object` is the object the frontend calls the hooks on, so only the object's name counts.
pub(super) fn build_hooks(root: &Path, build: &BuildConfiguration) -> Result<Vec<(PathBuf, String)>> {
    let Some(backend) = build.backend.as_deref() else {
        return Ok(Vec::new());
    };
    let (module, object) = backend.split_once(':').map_or((backend, None), |(module, object)| (module, Some(object)));
    let relative = module.trim().replace('.', "/");
    // Without a `backend-path`, the backend is looked for in the project's directory.
    let directories: Vec<PathBuf> = if build.backend_path.is_empty() {
        vec![root.to_path_buf()]
    } else {
        build.backend_path.iter().map(|directory| root.join(directory)).collect()
    };
    let Some(file) = directories
        .iter()
        .flat_map(|directory| [directory.join(format!("{relative}.py")), directory.join(&relative).join("__init__.py")])
        .find(|path| path.is_file())
    else {
        return Ok(Vec::new());
    };
    let file = std::path::absolute(&file).with_context(|| format!("Failed to resolve {}", file.display()))?;
    // The PEP 517 and PEP 660 hooks a build frontend calls by name, which no import of the project reaches.
    let names: Vec<String> = match object {
        Some(object) => vec![object.split('.').next().unwrap_or_default().trim().to_owned()],
        None => [
            "build_wheel",
            "build_sdist",
            "get_requires_for_build_wheel",
            "get_requires_for_build_sdist",
            "prepare_metadata_for_build_wheel",
            "build_editable",
            "get_requires_for_build_editable",
            "prepare_metadata_for_build_editable",
        ]
        .map(str::to_owned)
        .into(),
    };
    Ok(names.into_iter().map(|name| (file.clone(), name)).collect())
}
