use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Result, anyhow};
use colored::Colorize;
use ignore::Error as WalkError;
use log::warn;
use ruff_workspace::resolver::{ResolvedFile, Resolver, match_exclusion, project_files_in_path};
use rustc_hash::FxHashSet;

use super::Resolved;

// The files simplint checks, from Ruff's resolver, so the walk, `.gitignore`, the default and configured excludes and
// the per-file target version match `ruff check`. Stubs are not linted, but the names they declare are the API of
// their module. Files Ruff's `lint.exclude` leaves out are not linted either, yet the run still indexes them: a package
// `__init__.py` excluded there still imports and re-exports the names of the modules it covers.
pub(crate) struct Discovered<'a> {
    pub(crate) python: Vec<PathBuf>,
    pub(super) unlinted: Vec<PathBuf>,
    pub(crate) resolver: Resolver<'a>,
    pub(crate) missing: Vec<(PathBuf, String)>,
}

pub(crate) fn discover<'a>(files: &[PathBuf], configuration: &'a Resolved) -> Result<Discovered<'a>> {
    let (found, resolver) = project_files(files, configuration)?;
    let roots = linted_roots(files);
    let mut python = Vec::new();
    let mut unlinted = Vec::new();
    let mut missing = Vec::new();
    for file in found {
        let file = match file {
            Ok(file) => file,
            Err(error) => {
                match missing_path(&error) {
                    Some(path) => missing.push(path),
                    None => warn!("{} {error}", "Encountered error:".bold()),
                }
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
        let named = file.is_root() && !is_other_language(extension);
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
    Ok(Discovered { python: distinct(python), unlinted, resolver, missing })
}

fn project_files<'a>(
    files: &[PathBuf],
    configuration: &'a Resolved,
) -> Result<(Vec<Result<ResolvedFile, WalkError>>, Resolver<'a>)> {
    project_files_in_path(files, &configuration.pyproject_config, &configuration.config_arguments)
        .map_err(|error| redact_extended(error, &configuration.settings_resolver.root.root))
}

// Notebooks, TOML and Markdown files, which Ruff can lint but simplint does not.
fn is_other_language(extension: Option<&str>) -> bool {
    matches!(extension, Some("ipynb" | "toml" | "md"))
}

// The paths a run lints, canonical, so a file under them is one the run may read and write.
pub(crate) fn linted_roots(files: &[PathBuf]) -> Vec<PathBuf> {
    files.iter().filter_map(|file| fs::canonicalize(file).ok()).collect()
}

// The path and message of a walk error for a path the run names that does not exist, which Ruff reports as `io-error`.
fn missing_path(error: &WalkError) -> Option<(PathBuf, String)> {
    let WalkError::WithPath { path, err } = error else {
        return None;
    };
    let io = err.io_error().filter(|io| io.kind() == ErrorKind::NotFound)?;
    Some((path.clone(), io.to_string()))
}

// Whether a symlink points at a file outside the paths the run lints, which the run neither reads nor writes.
fn escapes_roots(path: &Path, roots: &[PathBuf]) -> bool {
    path.is_symlink()
        && path.is_file()
        && fs::canonicalize(path).is_ok_and(|target| !roots.iter().any(|root| target.starts_with(root)))
}

// The files sorted, with each file the links in the run reach kept once, by its own path when the run found it: files
// lint in parallel, so a fix applied through two paths to one file would race, and its reports would show twice. Hard
// links share a file's device and inode, which their canonical paths do not show.
fn distinct(mut python: Vec<PathBuf>) -> Vec<PathBuf> {
    python.sort_by_cached_key(|path| (path.is_symlink(), path.clone()));
    let mut seen = FxHashSet::default();
    python.retain(|path| identity(path).is_none_or(|identity| seen.insert(identity)));
    python.sort();
    python
}

#[cfg(unix)]
fn identity(path: &Path) -> Option<(u64, u64)> {
    use std::os::unix::fs::MetadataExt;
    fs::metadata(path).ok().map(|metadata| (metadata.dev(), metadata.ino()))
}

#[cfg(not(unix))]
fn identity(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

// Whether standard input, named by `--stdin-filename`, is linted as a file of that name on disk would be, except for a
// stub, which is linted here although discovery only indexes a `.pyi` file on disk. A notebook, TOML or Markdown file
// is not linted, and neither is a file Ruff's walk leaves out under `--force-exclude`. The name need not exist, so the
// walk reports it as missing rather than as a file.
pub(super) fn lints_stdin(path: &Path, configuration: &Resolved) -> Result<bool> {
    if is_other_language(path.extension().and_then(|extension| extension.to_str())) {
        return Ok(false);
    }
    let (found, resolver) = project_files(&[path.to_path_buf()], configuration)?;
    let settings = resolver.resolve(path);
    let excluded = settings.file_resolver.force_exclude
        && match_exclusion(path, path.file_name().unwrap_or_default(), &settings.linter.exclude);
    Ok(!found.is_empty() && !excluded)
}

// Ruff quotes the text of an `extend`ed file it cannot load. The repository may point `extend` anywhere, so such a file
// outside the project root is reported by name alone; inside the root it is part of the run, and Ruff's report stays.
// Ruff only normalizes the path, so a link in the repository could otherwise pass a file outside it as one inside.
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
        Some(existing.join(path.strip_prefix(ancestor).expect("an ancestor is a prefix of its path")))
    })
}

// Warns of the paths the run names that do not exist, for the commands that report no diagnostics of their own.
pub(crate) fn warn_missing(missing: &[(PathBuf, String)]) {
    for (_, message) in missing {
        warn!("{} {message}", "Encountered error:".bold());
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use anyhow::anyhow;

    use super::redact_extended;

    // A message that names no complete path is not one about an extended file, so it stays as Ruff wrote it.
    #[test]
    fn redact_extended_keeps_an_unterminated_name() {
        let message = "Failed to load extended configuration `/elsewhere/ruff.toml";

        let redacted = redact_extended(anyhow!(message), Path::new("."));

        assert_eq!(redacted.to_string(), message);
    }
}
