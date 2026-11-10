use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use colored::Colorize;
use log::warn;
use ruff_workspace::resolver::{Resolver, match_exclusion, project_files_in_path};

use super::Resolved;

// The files simplint checks, from Ruff's resolver, so the walk, `.gitignore`, the default and configured excludes and
// the per-file target version match `ruff check`. Stubs are not linted, but the names they declare are the API of
// their module. Files Ruff's `lint.exclude` leaves out are not linted either, yet the run still indexes them: a package
// `__init__.py` excluded there still imports and re-exports the names of the modules it covers.
pub(crate) struct Discovered<'a> {
    pub(crate) python: Vec<PathBuf>,
    pub(crate) unlinted: Vec<PathBuf>,
    pub(crate) resolver: Resolver<'a>,
}

pub(crate) fn discover<'a>(files: &[PathBuf], configuration: &'a Resolved) -> Result<Discovered<'a>> {
    let (found, resolver) =
        project_files_in_path(files, &configuration.pyproject_config, &configuration.config_arguments)
            .map_err(|error| redact_extended(error, &configuration.settings_resolver.root.root))?;
    let roots = linted_roots(files);
    let mut python = Vec::new();
    let mut unlinted = Vec::new();
    for file in found {
        let file = match file {
            Ok(file) => file,
            Err(error) => {
                warn!("{} {error}", "Encountered error:".bold());
                continue;
            }
        };
        let path = file.path();
        if escapes_roots(path, &roots) {
            warn!("{} {}: its target lies outside the linted paths", "Skipped symlink".bold(), path.display());
            continue;
        }
        let extension = path.extension().and_then(|extension| extension.to_str());
        if extension == Some("pyi") {
            unlinted.push(path.to_path_buf());
            continue;
        }
        // A file named on the command line is Python whatever its extension, as Ruff treats it, unless it is a
        // notebook, TOML or Markdown file, which simplint does not lint.
        let named = file.is_root() && !matches!(extension, Some("ipynb" | "toml" | "md"));
        if extension != Some("py") && !named {
            continue;
        }
        let settings = resolver.resolve(path);
        if (settings.file_resolver.force_exclude || !file.is_root())
            && match_exclusion(path, file.file_name(), &settings.linter.exclude)
        {
            unlinted.push(path.to_path_buf());
            continue;
        }
        python.push(path.to_path_buf());
    }
    python.sort();
    Ok(Discovered { python, unlinted, resolver })
}

// Ruff quotes the text of an `extend`ed file it cannot load. The repository may point `extend` anywhere, so such a file
// outside the project root is reported by name alone; inside the root it is part of the run, and Ruff's report stays. Ruff
// only normalizes the path, so a link in the repository could otherwise pass a file outside it as one inside.
pub(crate) fn redact_extended(error: anyhow::Error, project_root: &Path) -> anyhow::Error {
    let extended = error.chain().find_map(|cause| {
        let message = cause.to_string();
        let path = message.strip_prefix("Failed to load extended configuration `")?.split_once('`')?.0;
        Some(PathBuf::from(path))
    });
    match extended {
        Some(path) if !within(&path, project_root) => anyhow!("`{}` is not valid configuration", path.display()),
        _ => error,
    }
}

// Whether `path` lies inside `root` once the links in it resolve, so a link in the repository cannot point out of it.
pub(crate) fn within(path: &Path, root: &Path) -> bool {
    resolve_links(path).zip(fs::canonicalize(root).ok()).is_some_and(|(resolved, root)| resolved.starts_with(root))
}

// `path` with the links in its longest existing prefix resolved, as it may not exist yet.
fn resolve_links(path: &Path) -> Option<PathBuf> {
    path.ancestors().find_map(|ancestor| {
        let existing = fs::canonicalize(ancestor).ok()?;
        Some(existing.join(path.strip_prefix(ancestor).ok()?))
    })
}

// The paths a run lints, canonical, so a file under them is one the run may read and write.
pub(crate) fn linted_roots(files: &[PathBuf]) -> Vec<PathBuf> {
    files.iter().filter_map(|file| fs::canonicalize(file).ok()).collect()
}

// Whether a symlink points at a file outside the paths the run lints, which the run neither reads nor writes.
fn escapes_roots(path: &Path, roots: &[PathBuf]) -> bool {
    path.is_symlink()
        && path.is_file()
        && fs::canonicalize(path).is_ok_and(|target| !roots.iter().any(|root| target.starts_with(root)))
}
