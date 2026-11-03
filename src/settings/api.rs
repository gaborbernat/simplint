use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::path::Path;

use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_python_ast::helpers::is_dunder;
use serde::Deserialize;

use crate::settings::metadata::{BuildConfiguration, ProjectMetadata, import_names};

// What `project-kind` asks for; `auto` reads it from how the project builds.
#[derive(Deserialize, Clone, Copy, Default, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ProjectKindOption {
    #[default]
    Auto,
    Library,
    Application,
}

// A library has a Python API its users import; an application has none, so all of its code is internal.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProjectKind {
    Library,
    Application,
}

impl Display for ProjectKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Library => "library",
            Self::Application => "application",
        })
    }
}

// The docstring markup the `SMP3` rules check.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DocstringStyle {
    Rest,
}

impl Display for DocstringStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Rest => "rest",
        })
    }
}

// Which modules make up a library's public API: those of its API packages whose path has no `_` segment.
pub struct ApiScope {
    pub kind: ProjectKind,
    // The top-level packages and modules the distribution ships; `None` when none could be found, so every module
    // outside the documentation and the tests counts.
    pub packages: Option<BTreeSet<String>>,
    // Modules, besides the API packages themselves, whose `__all__` users import from directly.
    pub modules: Vec<Vec<String>>,
}

// Where a module sits in the public API: `public` when users may import from it, and `entry` when its `__all__` is the
// interface the author declares to them, rather than the names the rest of the project happens to use.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct ModuleRole {
    pub public: bool,
    pub entry: bool,
}

impl CacheKey for ModuleRole {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.public.cache_key(state);
        self.entry.cache_key(state);
    }
}

impl ApiScope {
    pub(super) fn resolve(
        root: &Path,
        option: ProjectKindOption,
        build: &BuildConfiguration,
        metadata: &ProjectMetadata,
        (packages, modules): (Option<&[String]>, Option<&[String]>),
    ) -> Self {
        let library =
            option == ProjectKindOption::Library || option == ProjectKindOption::Auto && is_installable(root, build);
        let kind = if library { ProjectKind::Library } else { ProjectKind::Application };
        let packages = packages.map_or_else(
            || shipped_packages(root, build, metadata),
            |names| Some(names.iter().map(|name| top_level(name).to_owned()).collect()),
        );
        let modules =
            modules.unwrap_or_default().iter().map(|module| module.split('.').map(str::to_owned).collect()).collect();
        Self { kind, packages, modules }
    }

    // A file's role from its module path, such as `["package", "module"]` or `["package", "__init__"]`; a file that is
    // not importable, a test module, and every module of an application have none.
    pub fn role(&self, module_path: Option<&[String]>, is_test: bool) -> ModuleRole {
        let Some(module_path) = module_path.filter(|_| self.kind == ProjectKind::Library && !is_test) else {
            return ModuleRole::default();
        };
        let path = match module_path.split_last() {
            Some((last, rest)) if last == "__init__" => rest,
            _ => module_path,
        };
        let shipped = path.first().is_some_and(|top| match &self.packages {
            Some(packages) => packages.contains(top),
            None => !matches!(top.as_str(), "docs" | "doc"),
        });
        let public = shipped && !path.iter().any(|part| part.starts_with('_') && !is_dunder(part));
        ModuleRole { public, entry: public && (path.len() == 1 || self.modules.iter().any(|module| module == path)) }
    }
}

impl CacheKey for ApiScope {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        (self.kind == ProjectKind::Library).cache_key(state);
        self.packages.iter().flatten().for_each(|package| package.cache_key(state));
        self.packages.is_some().cache_key(state);
        self.modules.cache_key(state);
    }
}

// Something builds and installs the project, so others can import it: a `[build-system]`, or the `setup.py` or
// `setup.cfg` pip falls back to without one. uv's `package = false` and maturin's binary-only builds install no Python
// package.
fn is_installable(root: &Path, build: &BuildConfiguration) -> bool {
    let declared = build.has_build_system || root.join("setup.py").is_file() || root.join("setup.cfg").is_file();
    declared && build.uv_package != Some(false) && build.maturin_bindings.as_deref() != Some("bin")
}

// The distribution's top-level packages, from the first source that names any: the build backend's settings, the
// packages that carry a `py.typed` marker, then the package named after the project.
fn shipped_packages(root: &Path, build: &BuildConfiguration, metadata: &ProjectMetadata) -> Option<BTreeSet<String>> {
    let configured: BTreeSet<String> = build.packages.iter().map(|name| top_level(name).to_owned()).collect();
    if !configured.is_empty() {
        return Some(configured);
    }
    let candidates = || {
        [root.to_path_buf(), root.join("src")]
            .into_iter()
            .filter_map(|directory| std::fs::read_dir(directory).ok())
            .flatten()
            .filter_map(Result::ok)
            .map(|entry| entry.path())
    };
    let typed: BTreeSet<String> = candidates()
        .filter(|path| path.join("py.typed").is_file())
        .filter_map(|path| path.file_name()?.to_str().map(str::to_owned))
        .collect();
    if !typed.is_empty() {
        return Some(typed);
    }
    let named: BTreeSet<String> = metadata
        .name
        .as_deref()
        .map(import_names)
        .unwrap_or_default()
        .into_iter()
        .filter(|name| {
            candidates().any(|path| {
                path.file_stem().and_then(|stem| stem.to_str()) == Some(name.as_str())
                    && (path.is_dir() || path.extension().is_some_and(|extension| extension == "py"))
            })
        })
        .collect();
    (!named.is_empty()).then_some(named)
}

// The top-level package of a dotted module name or of a path such as `src/package`.
fn top_level(name: &str) -> &str {
    let name = name.trim_end_matches(['/', '\\']);
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    name.split('.').next().unwrap_or(name)
}
