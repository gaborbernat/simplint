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

// Ruff handles file discovery and exclusions. Keep stubs and `lint.exclude` matches for the index alone so their
// declarations and re-exports still count.
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
        // Treat explicitly named files with unrecognized extensions as Python, including extensionless scripts.
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

// Prefer the original path when a symlink also reaches it. Otherwise parallel fixes could write the same file
// twice. File IDs also identify hard links on filesystems that provide them.
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

#[cfg(windows)]
#[derive(PartialEq, Eq, Hash)]
enum WindowsIdentity {
    File(u64, [u8; 16]),
    Path(PathBuf),
}

#[cfg(windows)]
fn identity(path: &Path) -> Option<WindowsIdentity> {
    use std::os::windows::{fs::OpenOptionsExt, io::AsRawHandle};

    use windows_sys::Win32::Storage::FileSystem::{
        FILE_ID_INFO, FILE_READ_ATTRIBUTES, FileIdInfo, GetFileInformationByHandleEx,
    };

    if let Ok(file) = fs::File::options().access_mode(FILE_READ_ATTRIBUTES).open(path) {
        let mut info = FILE_ID_INFO::default();
        // The handle stays open while Windows fills the buffer of the requested type.
        let ok = unsafe {
            GetFileInformationByHandleEx(
                file.as_raw_handle(),
                FileIdInfo,
                (&raw mut info).cast(),
                u32::try_from(size_of::<FILE_ID_INFO>()).expect("file ID information fits in a DWORD"),
            )
        };
        if ok != 0 && info.FileId.Identifier != [0; 16] {
            return Some(WindowsIdentity::File(info.VolumeSerialNumber, info.FileId.Identifier));
        }
    }
    fs::canonicalize(path).ok().map(WindowsIdentity::Path)
}

#[cfg(not(any(unix, windows)))]
fn identity(path: &Path) -> Option<PathBuf> {
    fs::canonicalize(path).ok()
}

// A stdin filename can name a stub or a file that does not yet exist. Ruff returns a missing-path entry for the
// latter. Both qualify unless the file filters exclude them.
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
