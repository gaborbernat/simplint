use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use anyhow::{Context, Result};
use serde::Deserialize;

// The `[project]` table of a `pyproject.toml`, reduced to what simplint's rules read: the project's name, the modules
// its unconditional requirements install, and the objects its scripts and entry points name.
#[derive(Deserialize, Default)]
#[serde(from = "ProjectTable")]
pub struct ProjectMetadata {
    pub(crate) name: Option<String>,
    pub(crate) dependencies: BTreeSet<String>,
    // Every module the project lists, whatever its marker, in optional dependencies and dependency groups too. A rule
    // asks whether a tool is declared at all, which an install may leave out.
    pub declared: BTreeSet<String>,
    // The modules an install of the project can bring, through its requirements and extras, whatever their marker.
    pub(crate) runtime: BTreeSet<String>,
    // Whether `[project]` lists the requirements itself, rather than leaving them to the build backend as `dynamic`.
    pub(crate) lists_dependencies: bool,
    pub(crate) entry_points: Vec<String>,
    pub(crate) scripts: Vec<String>,
}

impl ProjectMetadata {
    // `[dependency-groups]` sits outside `[project]`, so the parsed table cannot carry it. Its `include-group` tables
    // name no requirement and are skipped.
    pub(super) fn add_groups(&mut self, groups: &BTreeMap<String, Vec<toml::Value>>) {
        self.declared.extend(groups.values().flatten().filter_map(toml::Value::as_str).flat_map(import_names));
    }

    pub(super) fn add_tool_references(&mut self, pyproject: &toml::Table) {
        let lookup = |path: &[&str]| path.iter().try_fold(pyproject.get("tool")?, |value, key| value.get(key));
        if let Some(dynamic) = lookup(&["setuptools", "dynamic"]).and_then(toml::Value::as_table) {
            self.entry_points
                .extend(dynamic.values().filter_map(|value| value.get("attr")?.as_str().and_then(dotted_attribute)));
        }
        if let Some(classes) = lookup(&["setuptools", "cmdclass"]).and_then(toml::Value::as_table) {
            self.entry_points.extend(classes.values().filter_map(toml::Value::as_str).filter_map(dotted_attribute));
        }
        if let Some(groups) = lookup(&["poetry", "plugins"]).and_then(toml::Value::as_table) {
            self.entry_points.extend(
                groups
                    .values()
                    .filter_map(toml::Value::as_table)
                    .flat_map(toml::Table::values)
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned),
            );
        }
        self.entry_points.extend(strings(lookup(&["mypy", "plugins"])).into_iter().flat_map(|plugins| {
            plugins
                .split(',')
                .map(str::trim)
                .filter(|plugin| !plugin.is_empty())
                .map(|plugin| if plugin.contains(':') { plugin.to_owned() } else { format!("{plugin}:plugin") })
                .collect::<Vec<_>>()
        }));
        self.entry_points.extend(strings(lookup(&["pylint", "main", "load-plugins"])).into_iter().flat_map(
            |plugins| {
                plugins
                    .split(',')
                    .map(str::trim)
                    .filter(|plugin| !plugin.is_empty())
                    .map(|plugin| format!("{plugin}:register"))
                    .collect::<Vec<_>>()
            },
        ));
        for path in [&["pytest", "ini_options", "addopts"][..], &["pytest", "addopts"]] {
            let options: Vec<String> = strings(lookup(path))
                .into_iter()
                .flat_map(|option| option.split_whitespace().map(str::to_owned).collect::<Vec<_>>())
                .collect();
            self.entry_points.extend(
                options
                    .windows(2)
                    .filter(|pair| pair[0] == "-p")
                    .map(|pair| pair[1].as_str())
                    .chain(options.iter().filter_map(|option| option.strip_prefix("-p")))
                    .filter(|plugin| !plugin.is_empty() && !plugin.starts_with("no:"))
                    .map(str::to_owned),
            );
        }
    }
}

fn dotted_attribute(value: &str) -> Option<String> {
    let (module, attribute) = value.rsplit_once('.')?;
    Some(format!("{module}:{attribute}"))
}

impl From<ProjectTable> for ProjectMetadata {
    fn from(project: ProjectTable) -> Self {
        let scripts: Vec<String> = project.scripts.into_values().chain(project.gui_scripts.into_values()).collect();
        let lists_dependencies = project.name.is_some() && !project.dynamic.iter().any(|field| field == "dependencies");
        let runtime: BTreeSet<String> = project
            .dependencies
            .iter()
            .chain(project.optional_dependencies.values().flatten())
            .flat_map(|spec| import_names(spec))
            .collect();
        Self {
            name: project.name,
            // A requirement with an environment marker is conditional, so a guarded import of it is legitimate.
            dependencies: project
                .dependencies
                .iter()
                .filter(|spec| !spec.contains(';'))
                .flat_map(|spec| import_names(spec))
                .collect(),
            declared: runtime.clone(),
            runtime,
            lists_dependencies,
            entry_points: scripts
                .iter()
                .cloned()
                .chain(project.entry_points.into_values().flat_map(BTreeMap::into_values))
                .collect(),
            scripts,
        }
    }
}

pub(super) fn import_names(requirement: &str) -> Vec<String> {
    // Distributions whose import name differs from the project name; the rest import under their normalized name.
    const RENAMED: &[(&str, &str)] = &[
        ("attrs", "attr"),
        ("cffi", "_cffi_backend"),
        ("beautifulsoup4", "bs4"),
        ("opencv_python", "cv2"),
        ("opencv_python_headless", "cv2"),
        ("matplotlib", "mpl_toolkits"),
        ("pillow", "pil"),
        ("protobuf", "google"),
        ("pyjwt", "jwt"),
        ("python_dateutil", "dateutil"),
        ("python_dotenv", "dotenv"),
        ("pyyaml", "yaml"),
        ("pytest", "_pytest"),
        ("pywin32", "win32api"),
        ("pywin32", "win32con"),
        ("pywin32", "pythoncom"),
        ("pywin32", "pywintypes"),
        ("pyzmq", "zmq"),
        ("scikit_image", "skimage"),
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
    dynamic: Vec<String>,
    dependencies: Vec<String>,
    optional_dependencies: BTreeMap<String, Vec<String>>,
    scripts: BTreeMap<String, String>,
    gui_scripts: BTreeMap<String, String>,
    entry_points: BTreeMap<String, BTreeMap<String, String>>,
}

#[derive(Default)]
pub(super) struct BuildConfiguration {
    pub(super) has_build_system: bool,
    pub(crate) packages: Vec<String>,
    pub(super) uv_package: Option<bool>,
    pub(super) maturin_bindings: Option<String>,
    // The `build-backend` as written, `module` or `module:object`, and the `backend-path` directories it loads from.
    backend: Option<String>,
    backend_path: Vec<String>,
    // The `path` of each custom build or metadata hook of hatchling, which loads the hook's class from that file.
    pub(crate) hatch_hooks: Vec<String>,
    pdm_hook: Option<String>,
}

impl BuildConfiguration {
    pub(super) fn of(pyproject: &toml::Table, hatch_file: Option<&toml::Table>) -> Self {
        let lookup = |path: &[&str]| path.iter().try_fold(pyproject.get("tool")?, |value, key| value.get(key));
        let mut hatch = lookup(&["hatch"]).and_then(toml::Value::as_table).cloned().unwrap_or_default();
        if let Some(file) = hatch_file {
            overlay(&mut hatch, file);
        }
        let system_value = |key: &str| pyproject.get("build-system")?.as_table()?.get(key);
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
            hatch_hooks: hatch_hook_paths(&hatch),
            pdm_hook: lookup(&["pdm", "build", "custom-hook"]).and_then(toml::Value::as_str).map(str::to_owned),
        }
    }
}

// hatchling reads a `hatch.toml` next to `pyproject.toml` over the `[tool.hatch]` table, so its tables replace the ones
// of the same name.
fn overlay(base: &mut toml::Table, over: &toml::Table) {
    for (key, value) in over {
        match (base.get_mut(key), value) {
            (Some(toml::Value::Table(base_table)), toml::Value::Table(over_table)) => overlay(base_table, over_table),
            _ => _ = base.insert(key.clone(), value.clone()),
        }
    }
}

// The `path` of each custom build or metadata hook of a hatch table, and of each target's build hook. A hook that names
// no path loads `hatch_build.py` from the project's directory.
fn hatch_hook_paths(hatch: &toml::Table) -> Vec<String> {
    let build = hatch.get("build");
    let targets = build
        .and_then(|build| build.get("targets"))
        .and_then(toml::Value::as_table)
        .into_iter()
        .flat_map(toml::Table::values);
    [build, hatch.get("metadata")]
        .into_iter()
        .chain(targets.map(Some))
        .filter_map(|table| table?.get("hooks")?.get("custom"))
        .map(|hook| hook.get("path").and_then(toml::Value::as_str).unwrap_or("hatch_build.py").to_owned())
        .collect()
}

// The modules coverage loads as plugins, which it calls `coverage_init` on by name, and the directories their import
// path is searched in: the project's own, then the `pythonpath` of pytest and the `extra-paths` of ty, as the project
// configures them. Without a plugin there is nothing to search for, so no directory is kept.
#[derive(Default)]
pub struct CoveragePlugins {
    pub(crate) modules: Vec<String>,
    pub(crate) search_paths: Vec<PathBuf>,
}

impl CoveragePlugins {
    pub(super) fn of(pyproject: &toml::Table, project_dir: &Path) -> Self {
        let lookup = |path: &[&str]| path.iter().try_fold(pyproject.get("tool")?, |value, key| value.get(key));
        let modules = strings(lookup(&["coverage", "run", "plugins"]));
        if modules.is_empty() {
            return Self::default();
        }
        let search_paths = std::iter::once(project_dir.to_path_buf())
            .chain(
                [
                    &["pytest", "ini_options", "pythonpath"][..],
                    &["pytest", "pythonpath"],
                    &["ty", "environment", "extra-paths"],
                ]
                .iter()
                .flat_map(|path| strings(lookup(path)))
                .map(|directory| project_dir.join(directory)),
            )
            .map(|path| path.components().collect::<PathBuf>())
            // A configured directory a source distribution leaves out would fail the whole search path.
            .filter(|path| path.is_dir())
            .collect();
        Self { modules, search_paths }
    }
}

fn strings(value: Option<&toml::Value>) -> Vec<String> {
    match value {
        Some(toml::Value::String(text)) => vec![text.clone()],
        Some(toml::Value::Array(items)) => items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
}

// The files hatchling loads as its custom build and metadata hooks, each `path` relative to the project's directory.
// hatchling calls the class that subclasses its hook interface in such a file, so no import reaches it.
pub(super) fn hatch_hooks(root: &Path, build: &BuildConfiguration) -> Result<Vec<PathBuf>> {
    build
        .hatch_hooks
        .iter()
        .map(|path| {
            let file = root.join(path);
            let absolute =
                std::path::absolute(&file).with_context(|| format!("Failed to resolve {}", file.display()))?;
            Ok(normalized(&absolute))
        })
        .collect()
}

// The path hatchling makes of a hook's `path`, which `os.path.normpath` collapses: each `..` drops the component before
// it, and nothing is read from the file system, so a symbolic link keeps its place. The path is absolute, so a `..`
// past the root stays the root.
fn normalized(path: &Path) -> PathBuf {
    path.components().fold(PathBuf::new(), |mut normal, component| {
        match component {
            Component::ParentDir => _ = normal.pop(),
            _ => normal.push(component),
        }
        normal
    })
}

// The file and names of the build backend, each name paired with the file that defines it. A backend written as
// `module:object` is the object the frontend calls the hooks on, so only the object's name counts.
pub(super) fn build_hooks(root: &Path, build: &BuildConfiguration) -> Result<Vec<(PathBuf, String)>> {
    let Some(backend) = build.backend.as_deref() else {
        return Ok(Vec::new());
    };
    let mut hooks = Vec::new();
    if backend == "pdm.backend" {
        let file = root.join(build.pdm_hook.as_deref().unwrap_or("pdm_build.py"));
        if file.is_file() {
            let file = std::path::absolute(&file).with_context(|| format!("Failed to resolve {}", file.display()))?;
            hooks.extend(
                [
                    "pdm_build_hook_enabled",
                    "pdm_build_clean",
                    "pdm_build_initialize",
                    "pdm_build_update_setup_kwargs",
                    "pdm_build_update_files",
                    "pdm_build_finalize",
                ]
                .into_iter()
                .map(|name| (file.clone(), name.to_owned())),
            );
        }
    }
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
        return Ok(hooks);
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
    hooks.extend(names.into_iter().map(|name| (file.clone(), name)));
    Ok(hooks)
}
