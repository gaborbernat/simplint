use std::collections::{BTreeMap, BTreeSet};
use std::path::{Component, Path, PathBuf};

use itertools::Itertools;
use serde::Deserialize;

use crate::syntax::top_level;

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
    // Each unconditional requirement by its distribution name, with the modules it installs.
    pub(crate) requirements: Vec<(String, Vec<String>)>,
    pub(crate) entry_points: Vec<String>,
    pub(crate) scripts: Vec<String>,
    // Whether the build compiles Rust next to the Python code, which can look up any of its names by a string the run
    // does not see, as PyO3's imports do.
    pub(crate) native: bool,
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
            name: project.name.as_deref().map(canonical_name),
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
            requirements: project
                .dependencies
                .iter()
                .filter(|spec| !spec.contains(';'))
                .map(|spec| (distribution_name(spec).to_owned(), import_names(spec)))
                .collect(),
            entry_points: scripts
                .iter()
                .cloned()
                .chain(project.entry_points.into_values().flat_map(BTreeMap::into_values))
                .collect(),
            scripts,
            native: false,
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

// The name PyPI knows a distribution by, under which `Pkg`, `pkg` and `PKG` are one.
pub(crate) fn canonical_name(name: &str) -> String {
    name.split(['-', '_', '.']).filter(|part| !part.is_empty()).join("-").to_ascii_lowercase()
}

fn distribution_name(requirement: &str) -> &str {
    let end = requirement
        .find(|char: char| !(char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.')))
        .unwrap_or(requirement.len());
    &requirement[..end]
}

impl ProjectMetadata {
    // Whether an unconditional requirement installs the module under its exact name. Removing a guard around the import
    // relies on the answer, so a near match does not count.
    pub(crate) fn requires(&self, module: &str) -> bool {
        self.dependencies.contains(&top_level(module).to_ascii_lowercase())
    }

    // Whether a requirement or an extra may install the module. Reporting the module as undeclared relies on the
    // answer, so any likely match counts.
    pub(crate) fn may_install(&self, module: &str) -> bool {
        may_provide(&self.runtime, module)
    }

    // `[dependency-groups]` sits outside `[project]`, so the parsed table cannot carry it. Its `include-group` tables
    // name no requirement and are skipped.
    pub(super) fn add_groups(&mut self, groups: &BTreeMap<String, Vec<toml::Value>>) {
        self.declared.extend(groups.values().flatten().filter_map(toml::Value::as_str).flat_map(import_names));
    }

    pub(super) fn add_tool_references(&mut self, pyproject: &toml::Table) {
        if let Some(dynamic) = tool_value(pyproject, &["setuptools", "dynamic"]).and_then(toml::Value::as_table) {
            self.entry_points
                .extend(dynamic.values().filter_map(|value| dotted_attribute(value.get("attr")?.as_str()?)));
        }
        if let Some(classes) = tool_value(pyproject, &["setuptools", "cmdclass"]).and_then(toml::Value::as_table) {
            self.entry_points.extend(classes.values().filter_map(toml::Value::as_str).filter_map(dotted_attribute));
        }
        if let Some(groups) = tool_value(pyproject, &["poetry", "plugins"]).and_then(toml::Value::as_table) {
            self.entry_points.extend(
                groups
                    .values()
                    .filter_map(toml::Value::as_table)
                    .flat_map(toml::Table::values)
                    .filter_map(toml::Value::as_str)
                    .map(str::to_owned),
            );
        }
        let plugins = |path: &[&str]| {
            strings(tool_value(pyproject, path))
                .iter()
                .flat_map(|plugins| plugins.split(',').map(str::trim).filter(|plugin| !plugin.is_empty()))
                .map(str::to_owned)
                .collect::<Vec<_>>()
        };
        self.entry_points.extend(
            plugins(&["mypy", "plugins"])
                .into_iter()
                .map(|plugin| if plugin.contains(':') { plugin } else { format!("{plugin}:plugin") }),
        );
        self.entry_points.extend(
            plugins(&["pylint", "main", "load-plugins"]).into_iter().map(|plugin| format!("{plugin}:register")),
        );
    }
}

// A distribution's import name is its normalized name in most cases, so a namespace package such as `google.auth`
// matches `google-auth`, and one whose name adds a prefix or suffix, such as `grpcio` or `pyserial`, contains it.
pub(crate) fn may_provide<'a>(modules: impl IntoIterator<Item = &'a String> + Clone, module: &str) -> bool {
    let lower = module.to_ascii_lowercase();
    let mut prefix = String::new();
    for part in lower.split('.') {
        if !prefix.is_empty() {
            prefix.push('_');
        }
        prefix.push_str(part);
        if modules.clone().into_iter().any(|name| *name == prefix) {
            return true;
        }
    }
    let root = top_level(&lower);
    modules.into_iter().any(|name| name.contains(root))
}

// A value under `[tool]`, by the keys leading to it.
pub(super) fn tool_value<'a>(pyproject: &'a toml::Table, path: &[&str]) -> Option<&'a toml::Value> {
    path.iter().try_fold(pyproject.get("tool")?, |value, key| value.get(key))
}

fn dotted_attribute(value: &str) -> Option<String> {
    let (module, attribute) = value.rsplit_once('.')?;
    Some(format!("{module}:{attribute}"))
}

fn strings(value: Option<&toml::Value>) -> Vec<String> {
    match value {
        Some(toml::Value::String(text)) => vec![text.clone()],
        Some(toml::Value::Array(items)) => items.iter().filter_map(|item| item.as_str().map(str::to_owned)).collect(),
        _ => Vec::new(),
    }
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
    pub(super) native: bool,
}

impl BuildConfiguration {
    pub(super) fn of(pyproject: &toml::Table, hatch_file: Option<&toml::Table>) -> Self {
        let mut hatch = tool_value(pyproject, &["hatch"]).and_then(toml::Value::as_table).cloned().unwrap_or_default();
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
        .flat_map(|path| strings(tool_value(pyproject, path)))
        .collect();
        Self {
            has_build_system: pyproject.contains_key("build-system"),
            packages,
            uv_package: tool_value(pyproject, &["uv", "package"]).and_then(toml::Value::as_bool),
            maturin_bindings: tool_value(pyproject, &["maturin", "bindings"])
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            backend: system_value("build-backend").and_then(toml::Value::as_str).map(str::to_owned),
            backend_path: strings(system_value("backend-path")),
            hatch_hooks: hatch_hook_paths(&hatch),
            pdm_hook: tool_value(pyproject, &["pdm", "build", "custom-hook"])
                .and_then(toml::Value::as_str)
                .map(str::to_owned),
            native: strings(system_value("requires")).iter().any(|requirement| {
                matches!(canonical_name(distribution_name(requirement)).as_str(), "maturin" | "setuptools-rust")
            }),
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
    pub(super) fn of(pyproject: &toml::Table, project_dir: &Path, pythonpath: &[PathBuf]) -> Self {
        let modules = strings(tool_value(pyproject, &["coverage", "run", "plugins"]));
        if modules.is_empty() {
            return Self::default();
        }
        let search_paths = std::iter::once(project_dir.to_path_buf())
            .chain(pythonpath.iter().cloned())
            .chain(
                strings(tool_value(pyproject, &["ty", "environment", "extra-paths"]))
                    .into_iter()
                    .map(|directory| project_dir.join(directory)),
            )
            .map(|path| path.components().collect::<PathBuf>())
            // A configured directory a source distribution leaves out would fail the whole search path.
            .filter(|path| path.is_dir())
            .collect();
        Self { modules, search_paths }
    }
}

// The files hatchling loads as its custom build and metadata hooks, each `path` relative to the project's directory.
// hatchling calls the class that subclasses its hook interface in such a file, so no import reaches it.
pub(super) fn hatch_hooks(root: &Path, build: &BuildConfiguration) -> Vec<PathBuf> {
    build.hatch_hooks.iter().map(|path| normalized(&absolute(&root.join(path)))).collect()
}

// Resolving fails only for an empty path, and a path joined to the project's directory never is one.
fn absolute(path: &Path) -> PathBuf {
    std::path::absolute(path).expect("a path under the project directory is not empty")
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
pub(super) fn build_hooks(root: &Path, build: &BuildConfiguration) -> Vec<(PathBuf, String)> {
    let Some(backend) = build.backend.as_deref() else {
        return Vec::new();
    };
    let mut hooks = Vec::new();
    if backend == "pdm.backend" {
        let file = root.join(build.pdm_hook.as_deref().unwrap_or("pdm_build.py"));
        if file.is_file() {
            let file = absolute(&file);
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
        return hooks;
    };
    let file = absolute(&file);
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
    hooks
}

// The `__version__` a build backend or a release tool reads from the project's code, so the assignment has to stay
// where the tool looks: the files that setuptools' `attr:`, hatch's or pdm's version `path`, flit, bump-my-version,
// bump2version or tbump name, and every package's `__init__.py` when `setup.py` reads it itself.
#[derive(Default)]
pub struct VersionReaders {
    pub(crate) files: Vec<PathBuf>,
    pub(crate) any_init: bool,
}

impl VersionReaders {
    pub(super) fn of(project_dir: &Path, pyproject: Option<&toml::Table>) -> Self {
        let read = |name: &str| std::fs::read_to_string(project_dir.join(name)).unwrap_or_default();
        let setup_cfg = read("setup.cfg");
        let mut modules: Vec<String> = setup_cfg
            .lines()
            .filter_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == "version").then(|| value.trim().strip_prefix("attr:")).flatten()
            })
            .map(|attribute| attribute.trim().to_owned())
            .collect();
        let mut paths: Vec<String> = [setup_cfg.as_str(), read(".bumpversion.cfg").as_str()]
            .into_iter()
            .flat_map(str::lines)
            .filter_map(|line| line.trim().strip_prefix("[bumpversion:file:")?.strip_suffix(']').map(str::to_owned))
            .collect();
        let hatch = read("hatch.toml").parse::<toml::Table>().ok();
        paths.extend(hatch.as_ref().and_then(|hatch| hatch.get("version")?.get("path")?.as_str()).map(str::to_owned));
        let tbump = read("tbump.toml").parse::<toml::Table>().ok();
        let bump_my_version = read(".bumpversion.toml").parse::<toml::Table>().ok();
        let tables = pyproject.into_iter().chain(bump_my_version.as_ref()).chain(tbump.as_ref());
        for table in tables {
            modules.extend(
                tool_value(table, &["setuptools", "dynamic", "version", "attr"])
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned),
            );
            paths.extend(
                [&["hatch", "version", "path"][..], &["pdm", "version", "path"]]
                    .iter()
                    .filter_map(|path| tool_value(table, path)?.as_str().map(str::to_owned)),
            );
            let files = tool_value(table, &["bumpversion", "files"])
                .into_iter()
                .chain(tool_value(table, &["tbump", "file"]))
                .chain(table.get("file"))
                .filter_map(toml::Value::as_array)
                .flatten();
            paths.extend(
                files.filter_map(|file| file.get("filename").or_else(|| file.get("src"))?.as_str().map(str::to_owned)),
            );
        }
        if let Some(table) = pyproject {
            let backend =
                table.get("build-system").and_then(|system| system.get("build-backend")).and_then(toml::Value::as_str);
            let project = table.get("project");
            let dynamic_version = project
                .and_then(|project| project.get("dynamic"))
                .and_then(toml::Value::as_array)
                .is_some_and(|fields| fields.iter().any(|field| field.as_str() == Some("version")));
            if backend.is_some_and(|backend| backend.starts_with("flit")) && dynamic_version {
                let module = tool_value(table, &["flit", "module", "name"])
                    .and_then(toml::Value::as_str)
                    .map(str::to_owned)
                    .or_else(|| project?.get("name")?.as_str().map(|name| name.replace('-', "_")));
                modules.extend(module.map(|module| format!("{module}.__version__")));
            }
        }
        let module_files = modules.iter().flat_map(|attribute| {
            let module = attribute.rsplit_once('.').map_or(attribute.as_str(), |(module, _)| module).replace('.', "/");
            ["", "src/"]
                .into_iter()
                .flat_map(move |prefix| [format!("{prefix}{module}/__init__.py"), format!("{prefix}{module}.py")])
        });
        let files =
            paths.into_iter().chain(module_files).map(|path| normalized(&absolute(&project_dir.join(path)))).collect();
        Self { files, any_init: read("setup.py").contains("__version__") }
    }
}
