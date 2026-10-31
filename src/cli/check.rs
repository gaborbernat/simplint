// `simplint check`: the files it covers, the import index built over them, and each file's linter.
use std::borrow::Cow;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, bail};
use colored::Colorize;
use itertools::Itertools;
use log::{debug, error, warn};
use rayon::prelude::*;
use ruff::ExitStatus;
use ruff::args::ConfigArguments;
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::logging::LogLevel;
use ruff_linter::package::PackageRoot;
use ruff_linter::packaging::detect_package_root;
use ruff_linter::settings::flags;
use ruff_linter::{SuppressionKind, warn_user, warn_user_once};
use ruff_python_ast::helpers::to_module_path;
use ruff_workspace::resolver::{PyprojectConfig, Resolver, match_exclusion, project_files_in_path};
use rustc_hash::FxHashMap;

use crate::cli::args::{CheckCommand, GlobalConfigArgs, suppression_flag};
use crate::cli::diagnostics::{Diagnostics, Mode, lint_path, lint_stdin};
use crate::cli::printer::{Flags, Printer};
use crate::cli::watch::watch;
use crate::lint::Linter;
use crate::project::cache::{self, Cache, Caches, FileKey, Fingerprints};
use crate::project::{FileFacts, Project, SearchRoots};
use crate::settings::{FileSettings, Settings, SettingsResolver, ShownSettings};

// `ruff check`'s flow: resolve both configurations, decide the fix mode, lint every file in parallel, print, and pick
// the exit status.
pub(crate) fn check(cli: &CheckCommand, global_options: &GlobalConfigArgs) -> Result<ExitStatus> {
    let log_level = global_options.log_level();
    let resolved = Resolved::new(cli, global_options)?;
    let is_stdin = cli.files.first().is_some_and(|file| file == Path::new("-"))
        || (cli.files.is_empty() && cli.stdin_filename.is_some());
    let files = if cli.files.is_empty() || is_stdin { vec![PathBuf::from(".")] } else { cli.files.clone() };
    let mut writer: Box<dyn Write> = match &cli.output_file {
        Some(path) if !cli.watch => {
            colored::control::set_override(false);
            path.parent().map(std::fs::create_dir_all).transpose()?;
            Box::new(BufWriter::new(std::fs::File::create(path)?))
        }
        _ => Box::new(BufWriter::new(std::io::stdout())),
    };
    if cli.show_settings {
        show_settings(&files, &resolved, &mut writer)?;
        writer.flush()?;
        return Ok(ExitStatus::Success);
    }
    if cli.show_files {
        show_files(&files, &resolved, &mut writer)?;
        writer.flush()?;
        return Ok(ExitStatus::Success);
    }
    let run = &resolved.settings_resolver.root.run;
    let fix_only = run.fix_only.unwrap_or(false);
    let mode = Mode {
        fix: if cli.diff {
            flags::FixMode::Diff
        } else if run.fix.unwrap_or(false) || fix_only {
            flags::FixMode::Apply
        } else {
            flags::FixMode::Generate
        },
        unsafe_fixes: run.unsafe_fixes(),
        noqa: if cli.ignore_noqa { flags::Noqa::Disabled } else { flags::Noqa::Enabled },
    };
    if let Some((reason, kind)) = cli.suppression() {
        let flag = suppression_flag(kind);
        if !mode.fix.is_generate() {
            warn_user!("--fix is incompatible with {flag}.");
        }
        if reason.contains(['\n', '\r']) {
            bail!("{flag} <reason> cannot contain newline characters");
        }
        let modifications = add_suppressions(&files, &resolved, kind, (!reason.is_empty()).then_some(reason))?;
        if modifications > 0 && log_level >= LogLevel::Default {
            let noun = match kind {
                SuppressionKind::Noqa => "noqa directive",
                SuppressionKind::Ignore => "ignore comment",
            };
            let plural = if modifications == 1 { "" } else { "s" };
            eprintln!("Added {modifications} {noun}{plural}.");
        }
        return Ok(ExitStatus::Success);
    }
    let mut printer_flags = Flags::empty();
    if !(cli.diff || fix_only) {
        printer_flags |= Flags::SHOW_VIOLATIONS;
    }
    if run.show_fixes.unwrap_or(false) {
        printer_flags |= Flags::SHOW_FIX_SUMMARY;
    }
    let printer = Printer {
        format: run.output_format.unwrap_or_default(),
        log_level,
        mode: &mode,
        flags: printer_flags,
        preview: resolved.pyproject_config.settings.linter.preview.is_enabled(),
        prefer_rule_codes: resolved.pyproject_config.settings.output_prefer_rule_codes,
    };
    // As in Ruff, the cache is only read and written while `noqa` comments apply.
    let use_cache = !cli.no_cache && mode.noqa.is_enabled();
    if cli.watch {
        return watch(cli, global_options, &files, resolved, use_cache, &printer, writer);
    }
    let mut diagnostics =
        if is_stdin { check_stdin(cli, &resolved, &mode)? } else { check_files(&files, &resolved, &mode, use_cache)? };
    diagnostics.inner.sort_by(Diagnostic::ruff_start_ordering);
    // With fixed source going to standard output, the report goes to standard error.
    if is_stdin && !mode.fix.is_generate() && cli.output_file.is_none() {
        writer = Box::new(std::io::stderr());
    }
    if cli.statistics {
        printer.write_statistics(&diagnostics, &mut writer)?;
    } else {
        printer.write_once(&diagnostics, &mut writer)?;
    }
    writer.flush()?;
    Ok(exit_status(cli, &diagnostics, fix_only))
}

// Ruff's configuration and simplint's, from the same command line.
pub(super) struct Resolved {
    config_arguments: ConfigArguments,
    pub(super) settings_resolver: SettingsResolver,
    pub(super) pyproject_config: PyprojectConfig,
}

impl Resolved {
    pub(super) fn new(cli: &CheckCommand, global_options: &GlobalConfigArgs) -> Result<Self> {
        let (config_arguments, arguments) = cli.partition(global_options)?;
        let settings_resolver =
            SettingsResolver::new(arguments.config_file.as_deref(), arguments.isolated, arguments.overrides)?;
        let pyproject_config = ruff::resolve::resolve(&config_arguments, cli.stdin_filename.as_deref())?;
        Ok(Self { config_arguments, settings_resolver, pyproject_config })
    }
}

fn show_settings(files: &[PathBuf], resolved: &Resolved, writer: &mut dyn Write) -> Result<()> {
    let Discovered { python, resolver, .. } = discover(files, resolved)?;
    let Some(path) = python.first() else {
        bail!("No files found under the given path");
    };
    let settings = resolved.settings_resolver.for_file(path)?;
    writeln!(writer, "Resolved settings for: {:?}", path.display().to_string())?;
    if let Some(settings_path) = &settings.path {
        writeln!(writer, "Settings path: {:?}", settings_path.display().to_string())?;
    }
    let settings = FileSettings::new(&settings, resolver.resolve(path), path);
    let cache_dir = cache::directory(&resolved.pyproject_config.settings.cache_dir);
    write!(writer, "{}", ShownSettings { settings, cache_dir: &cache_dir })?;
    Ok(())
}

// The files simplint checks, from Ruff's resolver, so the walk, `.gitignore`, the default and configured excludes and
// the per-file target version match `ruff check`. Stubs are not linted, but the names they declare are the API of
// their module.
struct Discovered<'a> {
    python: Vec<PathBuf>,
    stubs: Vec<PathBuf>,
    resolver: Resolver<'a>,
}

fn discover<'a>(files: &[PathBuf], configuration: &'a Resolved) -> Result<Discovered<'a>> {
    let (found, resolver) =
        project_files_in_path(files, &configuration.pyproject_config, &configuration.config_arguments)?;
    let mut python = Vec::new();
    let mut stubs = Vec::new();
    for file in found {
        let file = match file {
            Ok(file) => file,
            Err(error) => {
                warn!("{} {error}", "Encountered error:".bold());
                continue;
            }
        };
        let path = file.path();
        let extension = path.extension().and_then(|extension| extension.to_str());
        if extension == Some("pyi") {
            stubs.push(path.to_path_buf());
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
            continue;
        }
        python.push(path.to_path_buf());
    }
    python.sort();
    Ok(Discovered { python, stubs, resolver })
}

fn show_files(files: &[PathBuf], resolved: &Resolved, writer: &mut dyn Write) -> Result<()> {
    let discovered = discover(files, resolved)?;
    if discovered.python.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
    }
    for path in &discovered.python {
        writeln!(writer, "{}", path.to_string_lossy())?;
    }
    Ok(())
}

// Ruff's `add_noqa`: a file that cannot be read or written is logged and left as it is.
fn add_suppressions(
    files: &[PathBuf],
    resolved: &Resolved,
    kind: SuppressionKind,
    reason: Option<&str>,
) -> Result<usize> {
    let counts = for_each_file(files, resolved, None, |DiscoveredFile { path, linter, .. }| {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                error!("Failed to extract source from {}: {error}", path.display());
                return 0;
            }
        };
        let (count, output) = linter.add_suppressions(path, &source, kind, reason);
        if count > 0
            && let Err(error) = std::fs::write(path, output)
        {
            error!("Failed to add suppression to {}: {error}", path.display());
            return 0;
        }
        count
    })?;
    Ok(counts.into_iter().flatten().sum())
}

// One discovered Python file, with its linter, and its package's cache with its key when the cache is on.
struct DiscoveredFile<'a> {
    path: &'a Path,
    linter: Linter<'a>,
    cache: Option<(&'a Cache, FileKey)>,
}

// Ruff's per-file loop over the discovered files, with the import index of the whole run built first, and the caches
// in `cache_dir` read before and written after. `None` when there is no Python file to check.
fn for_each_file<T: Send>(
    files: &[PathBuf],
    resolved: &Resolved,
    cache_dir: Option<&Path>,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Option<Vec<T>>> {
    let start = Instant::now();
    let Discovered { python, stubs, resolver } = discover(files, resolved)?;
    debug!("Identified files to lint in: {:?}", start.elapsed());
    if python.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
        return Ok(None);
    }
    let settings: Vec<(PathBuf, Arc<Settings>)> = python
        .into_iter()
        .map(|path| Ok((path.clone(), resolved.settings_resolver.for_file(&path)?)))
        .collect::<Result<_>>()?;
    let paths: Vec<&Path> =
        settings.iter().map(|(path, _)| path.as_path()).chain(stubs.iter().map(PathBuf::as_path)).collect();
    let entry_points: Vec<String> = settings
        .iter()
        .map(|(_, settings)| settings)
        .unique_by(|settings| Arc::as_ptr(settings))
        .flat_map(|settings| settings.metadata.entry_points.iter().cloned())
        .sorted_unstable()
        .dedup()
        .collect();
    let packages = resolver.package_roots(&paths);
    let roots = SearchRoots::new(&resolved.pyproject_config.settings, packages.values().flatten().copied());
    let package = |path: &Path| path.parent().and_then(|parent| packages.get(parent)).copied().flatten();
    let module_path = |path: &Path| package(path).and_then(|package| to_module_path(package.path(), path));
    let caches = cache_dir.map(|cache_dir| {
        Caches::open(cache_dir, packages.iter().map(|(parent, package)| package.map_or(*parent, PackageRoot::path)))
    });
    // As in Ruff, a file's cache is its package's, or its directory's outside a package.
    let file_caches: FxHashMap<&Path, (&Cache, FileKey)> = caches.as_ref().map_or_else(FxHashMap::default, |caches| {
        paths
            .par_iter()
            .filter_map(|&path| {
                let root = package(path).map(PackageRoot::path).or_else(|| path.parent())?;
                Some((path, (caches.get(root)?, FileKey::of(path)?)))
            })
            .collect()
    });
    let cache = |path: &Path| file_caches.get(path).copied();
    let project = index_project(&paths, &entry_points, &roots, module_path, cache)?;
    let start = Instant::now();
    let results: Vec<T> = settings
        .par_iter()
        .map(|(path, settings)| {
            let linter = file_linter(path, settings, resolver.resolve(path), &project, package(path));
            work(&DiscoveredFile { path, linter, cache: cache(path) })
        })
        .collect();
    debug!("Checked {:?} files in: {:?}", results.len(), start.elapsed());
    if let Some(caches) = caches {
        caches.persist()?;
    }
    Ok(Some(results))
}

// The import index of the whole run, from each file's facts: those its cache holds while the file is unchanged, and
// otherwise freshly collected and stored there.
fn index_project<'c>(
    paths: &[&Path],
    entry_points: &[String],
    roots: &SearchRoots,
    module_path: impl Fn(&Path) -> Option<Vec<String>> + Sync,
    cache: impl Fn(&Path) -> Option<(&'c Cache, FileKey)> + Sync,
) -> Result<Project> {
    let start = Instant::now();
    let facts: Vec<(&Path, Cow<FileFacts>)> = paths
        .par_iter()
        .filter_map(|&path| {
            let cache = cache(path);
            if let Some(facts) = cache.and_then(|(cache, key)| cache.facts(path, key)) {
                return Some((path, Cow::Borrowed(facts)));
            }
            let facts = FileFacts::collect(path, module_path(path).as_deref())?;
            if let Some((cache, key)) = cache {
                cache.set_facts(path, key, facts.clone());
            }
            Some((path, Cow::Owned(facts)))
        })
        .collect();
    let indexed: Vec<(&Path, &FileFacts)> = facts.iter().map(|(path, facts)| (*path, facts.as_ref())).collect();
    let project = Project::index(&indexed, entry_points, roots)?;
    debug!("Indexed {} files in: {:?}", indexed.len(), start.elapsed());
    Ok(project)
}

// A file's linter, from simplint's settings and Ruff's for it and the package it sits in.
fn file_linter<'a>(
    path: &Path,
    settings: &'a Settings,
    ruff: &'a ruff_workspace::Settings,
    project: &'a Project,
    package: Option<PackageRoot<'a>>,
) -> Linter<'a> {
    let module_path = package.and_then(|package| to_module_path(package.path(), path));
    Linter::new(ruff, FileSettings::new(settings, ruff, path), project.file(path), package, module_path)
}

// Ruff's `check_stdin`: the source comes from standard input, and nothing else is indexed. Otherwise it is linted as
// the file it stands for would be, so its settings, package and search roots come from that file's absolute path.
fn check_stdin(cli: &CheckCommand, resolved: &Resolved, mode: &Mode) -> Result<Diagnostics> {
    let ruff = &resolved.pyproject_config.settings;
    let path = cli.stdin_filename.clone().unwrap_or_else(|| PathBuf::from("-"));
    let absolute = std::path::absolute(&path)?;
    let settings = resolved.settings_resolver.for_file(&absolute)?;
    let mut source = String::new();
    std::io::stdin().read_to_string(&mut source)?;
    let package = absolute
        .parent()
        .and_then(|parent| detect_package_root(parent, &ruff.linter.namespace_packages))
        .map(PackageRoot::root);
    let project = Project::index(&[], &settings.metadata.entry_points, &SearchRoots::new(ruff, package))?;
    let linter = file_linter(&absolute, &settings, ruff, &project, package);
    lint_stdin(&path, &source, &linter, mode)
}

pub(super) fn check_files(files: &[PathBuf], resolved: &Resolved, mode: &Mode, use_cache: bool) -> Result<Diagnostics> {
    let cache_dir = use_cache.then_some(resolved.pyproject_config.settings.cache_dir.as_path());
    let fingerprints = Fingerprints::default();
    let results = for_each_file(files, resolved, cache_dir, |file| {
        let lint = |settled| lint_path(file.path, &file.linter, mode, settled);
        let Some((cache, key)) = file.cache else {
            return lint(None).0;
        };
        cache.lint_or_reuse(file.path, key, fingerprints.of(&file.linter, mode), lint)
    })?;
    Ok(results.into_iter().flatten().fold(Diagnostics::default(), |left, right| left + right))
}

fn exit_status(cli: &CheckCommand, diagnostics: &Diagnostics, fix_only: bool) -> ExitStatus {
    let failed = if cli.exit_zero {
        false
    } else if cli.diff || (fix_only && cli.exit_non_zero_on_fix) {
        !diagnostics.fixed.is_empty()
    } else if fix_only {
        false
    } else if cli.exit_non_zero_on_fix {
        !diagnostics.fixed.is_empty() || !diagnostics.inner.is_empty()
    } else {
        !diagnostics.inner.is_empty()
    };
    if failed { ExitStatus::Failure } else { ExitStatus::Success }
}
