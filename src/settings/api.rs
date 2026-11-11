use std::collections::BTreeSet;
use std::fmt::{Display, Formatter};
use std::path::Path;

use ruff_cache::{CacheKey, CacheKeyHasher};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::settings::metadata::{BuildConfiguration, ProjectMetadata, import_names};
use crate::syntax::Visibility;

// What `project-kind` asks for; `auto` reads it from how the project builds.
#[derive(Deserialize, Clone, Copy, Default, PartialEq, Eq, JsonSchema)]
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

// The docstring markup the reStructuredText rules check.
#[derive(Deserialize, Clone, Copy, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum DocstringStyle {
    Rst,
}

impl Display for DocstringStyle {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Rst => "rst",
        })
    }
}

// Which modules make up a library's public API: those of its API packages whose path has no `_` segment.
pub struct ApiScope {
    pub(crate) kind: ProjectKind,
    // The top-level packages and modules the distribution ships; `None` when none could be found, so every module
    // outside the documentation and the tests counts.
    pub(crate) packages: Option<BTreeSet<String>>,
}

// Where a module sits in the public API: `public` when users may import from it. The `__all__` of a public module is
// the interface its author declares to them, so no name in it needs an importer in the project.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct ModuleRole {
    pub(crate) public: bool,
}

impl CacheKey for ModuleRole {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        self.public.cache_key(state);
    }
}

impl ApiScope {
    pub(super) fn resolve(
        root: &Path,
        option: ProjectKindOption,
        build: &BuildConfiguration,
        metadata: &ProjectMetadata,
        packages: Option<&[String]>,
    ) -> Self {
        let library =
            option == ProjectKindOption::Library || option == ProjectKindOption::Auto && is_installable(root, build);
        let kind = if library { ProjectKind::Library } else { ProjectKind::Application };
        let packages = packages.map_or_else(
            || shipped_packages(root, build, metadata),
            |names| Some(names.iter().map(|name| package_name(name)).collect()),
        );
        Self { kind, packages }
    }

    // A file's role from its module path, such as `["package", "module"]` or `["package", "__init__"]`; a file that is
    // not importable, a test module, and every module of an application have none.
    pub(crate) fn role(&self, module_path: Option<&[String]>, is_test: bool) -> ModuleRole {
        let Some(module_path) = module_path.filter(|_| self.kind == ProjectKind::Library && !is_test) else {
            return ModuleRole::default();
        };
        let path = match module_path.split_last() {
            Some((last, rest)) if last == "__init__" => rest,
            _ => module_path,
        };
        // The most specific API package containing the module, so a nested package wins over its parent.
        let root = match &self.packages {
            Some(packages) => packages
                .iter()
                .map(|package| package.split('.').collect::<Vec<_>>())
                .filter(|parts| {
                    parts.len() <= path.len() && parts.iter().zip(path).all(|(part, segment)| part == segment)
                })
                .map(|parts| parts.len())
                .max(),
            None => path.first().filter(|top| !matches!(top.as_str(), "docs" | "doc")).map(|_| 1),
        };
        // A dunder module such as `__about__` is public, but `__main__` runs as a script rather than being imported.
        let public = root.is_some()
            && !path.iter().any(|part| Visibility::of(part) == Visibility::Private || part == "__main__");
        ModuleRole { public }
    }
}

impl CacheKey for ApiScope {
    fn cache_key(&self, state: &mut CacheKeyHasher) {
        (self.kind == ProjectKind::Library).cache_key(state);
        self.packages.iter().flatten().for_each(|package| package.cache_key(state));
        self.packages.is_some().cache_key(state);
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
    let configured: BTreeSet<String> = build.packages.iter().map(|name| package_name(name)).collect();
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

// A package by its dotted import name: the last part of a path such as `src/package`, which a build backend ships at
// the top level, or a dotted name such as `namespace.package`, without trailing private modules, as the `_native` of
// maturin's `package._native`.
fn package_name(name: &str) -> String {
    let name = name.trim_end_matches(['/', '\\']);
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let parts: Vec<&str> = name.split('.').collect();
    let kept =
        parts.iter().rposition(|part| Visibility::of(part) == Visibility::Public).map_or(parts.len(), |last| last + 1);
    parts[..kept].join(".")
}
