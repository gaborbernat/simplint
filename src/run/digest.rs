use std::hash::Hasher;
use std::path::{Path, PathBuf};

use itertools::Itertools;
use ruff_cache::{CacheKey, CacheKeyHasher};
use ruff_linter::package::PackageRoot;

use super::files::{Documentation, ProjectInputs, RunFile};
use crate::lint::registry::Rule;
use crate::project::cache::FileKey;
use crate::project::{DocReferences, Parts, SearchRoots};
use crate::settings::ModuleRole;

// A cache entry stored under the same digest was computed from the same index, which then need not be built.
#[expect(clippy::too_many_arguments, reason = "the run's inputs, which the digest hashes")]
pub(super) fn run_digest(
    files: &[RunFile],
    keys: &[Option<FileKey>],
    linted: usize,
    roots: &SearchRoots,
    startup: &[String],
    projects: &[ProjectInputs],
    docs: &Documentation,
    parts: Parts,
) -> u64 {
    let mut hasher = CacheKeyHasher::new();
    std::env::current_dir().unwrap_or_default().cache_key(&mut hasher);
    roots.cache_key(&mut hasher);
    roots.layout_key(files.iter().map(|file| file.path), &mut hasher);
    linted.cache_key(&mut hasher);
    for (file, key) in files.iter().zip(keys) {
        file.path.cache_key(&mut hasher);
        key.cache_key(&mut hasher);
        file.package.map(PackageRoot::path).map(Path::to_path_buf).cache_key(&mut hasher);
        file.module_path.cache_key(&mut hasher);
        file.importable.cache_key(&mut hasher);
        file.nested.cache_key(&mut hasher);
    }
    startup.cache_key(&mut hasher);
    for project in projects {
        project.entry_points.cache_key(&mut hasher);
        project.build_hooks.cache_key(&mut hasher);
        project.coverage_plugins.cache_key(&mut hasher);
        project.coverage_paths.cache_key(&mut hasher);
        project.duplicate_statements.cache_key(&mut hasher);
        project.probed.cache_key(&mut hasher);
        let mut test_modules: Vec<&Path> = project.test_modules.iter().copied().collect();
        test_modules.sort_unstable();
        test_modules.cache_key(&mut hasher);
        let mut dependency_hosts: Vec<&Path> = project.dependency_hosts.iter().copied().collect();
        dependency_hosts.sort_unstable();
        dependency_hosts.cache_key(&mut hasher);
        let mut distribution_roots: Vec<(&Path, &Path)> =
            project.distribution_roots.iter().map(|(path, root)| (*path, *root)).collect();
        distribution_roots.sort_unstable_by_key(|(path, _)| *path);
        for (path, root) in distribution_roots {
            path.cache_key(&mut hasher);
            root.cache_key(&mut hasher);
        }
        let mut roles: Vec<(&Path, ModuleRole)> = project.roles.iter().map(|(path, role)| (*path, *role)).collect();
        roles.sort_unstable_by_key(|(path, _)| *path);
        for (path, role) in roles {
            path.cache_key(&mut hasher);
            role.cache_key(&mut hasher);
        }
    }
    let mut documented: Vec<(&Option<PathBuf>, &DocReferences)> = docs.iter().collect();
    documented.sort_unstable_by_key(|(project, _)| *project);
    for (project, references) in documented {
        project.cache_key(&mut hasher);
        references.cache_key(&mut hasher);
    }
    parts.cache_key(&mut hasher);
    hasher.finish()
}

// Read only by `private-import-path`, whose paired-test lookup needs the layout beside each project. `None` when no
// linted file checks for it.
pub(super) fn probed_layout_key(files: &[&RunFile]) -> Option<u64> {
    let mut roots: Vec<&Path> = files
        .iter()
        .filter(|file| file.settings.rules.contains(Rule::PrivateImportPath))
        .map(|file| file.settings.project.root.as_path())
        .unique()
        .collect();
    if roots.is_empty() {
        return None;
    }
    roots.sort_unstable();
    let mut hasher = CacheKeyHasher::new();
    for root in roots {
        SearchRoots::probed_layout(root, &mut hasher);
    }
    Some(hasher.finish())
}
