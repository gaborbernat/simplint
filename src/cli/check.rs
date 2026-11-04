// `simplint check`: the files it covers, the import index built over them, and each file's linter.
use rustc_hash::{FxHashMap, FxHashSet};
use std::borrow::Cow;
use std::fs;
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Result, anyhow, bail};
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
use ruff_python_ast::PythonVersion;
use ruff_python_ast::helpers::to_module_path;
use ruff_workspace::resolver::{PyprojectConfig, Resolver, match_exclusion, project_files_in_path};

use crate::cli::args::{CheckCommand, GlobalConfigArgs, suppression_flag};
use crate::cli::conflicts::{self, Scope};
use crate::cli::diagnostics::{Diagnostics, Mode, lint_path, lint_stdin, write_source};
use crate::cli::printer::{Flags, Printer};
use crate::cli::watch::watch;
use crate::lint::Linter;
use crate::lint::registry::RuleSet;
use crate::project::cache::{self, Cache, Caches, FileKey, Fingerprints};
use crate::project::{FileFacts, ImportResolver, IndexInputs, Project, SearchRoots};
use crate::settings::{FileSettings, ModuleRole, Settings, SettingsResolver};

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
        roots: linted_roots(&files),
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
    // Files are linted in parallel, so their diffs wait here to print in the order the report sorts files.
    diagnostics.diffs.sort_by(|left, right| left.0.cmp(&right.0));
    for (_, diff) in &diagnostics.diffs {
        write!(std::io::stdout().lock(), "{diff}")?;
    }
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
    pub(super) cache_dir: PathBuf,
}

impl Resolved {
    pub(super) fn new(cli: &CheckCommand, global_options: &GlobalConfigArgs) -> Result<Self> {
        let (config_arguments, arguments) = cli.partition(global_options)?;
        let settings_resolver =
            SettingsResolver::new(arguments.config_file.as_deref(), arguments.isolated, arguments.overrides)?;
        let pyproject_config = ruff::resolve::resolve(&config_arguments, cli.stdin_filename.as_deref())
            .map_err(|error| redact_extended(error, &settings_resolver.root.root))?;
        let cache_dir = choose_cache_dir(cli, &pyproject_config);
        Ok(Self { config_arguments, settings_resolver, pyproject_config, cache_dir })
    }

    // Ruff's `output-prefer-rule-codes`: in preview, messages name rules unless it is set.
    fn names_rules(&self) -> bool {
        self.pyproject_config.settings.linter.preview.is_enabled()
            && !self.pyproject_config.settings.output_prefer_rule_codes
    }
}

// Ruff's cache directory, where simplint keeps its cache. A `cache-dir` in a configuration file comes from the
// repository being linted, so it must stay inside the project root, the directory of that file; one that leaves it
// falls back to Ruff's default there. A path from the command line or `RUFF_CACHE_DIR` is the user's choice and stands
// wherever it points.
fn choose_cache_dir(cli: &CheckCommand, config: &PyprojectConfig) -> PathBuf {
    let configured = &config.settings.cache_dir;
    match config.path.as_deref().and_then(Path::parent) {
        Some(root) if cli.cache_dir.is_none() && !within(configured, root) => ruff_cache::cache_dir(root),
        _ => configured.clone(),
    }
}

// Whether `path` lies inside `root` once the links in it resolve, so a link in the repository cannot point the cache
// out of it.
fn within(path: &Path, root: &Path) -> bool {
    resolve_links(path).zip(fs::canonicalize(root).ok()).is_some_and(|(resolved, root)| resolved.starts_with(root))
}

// `path` with the links in its longest existing prefix resolved, as the cache directory may not exist yet.
fn resolve_links(path: &Path) -> Option<PathBuf> {
    path.ancestors().find_map(|ancestor| {
        let existing = fs::canonicalize(ancestor).ok()?;
        Some(existing.join(path.strip_prefix(ancestor).ok()?))
    })
}

// Ruff quotes the text of an `extend`ed file it cannot load. The repository may point `extend` anywhere, so such a file
// outside the project root is reported by name alone; inside the root it is part of the run, and Ruff's report stays.
fn redact_extended(error: anyhow::Error, project_root: &Path) -> anyhow::Error {
    let extended = error.chain().find_map(|cause| {
        let message = cause.to_string();
        let path = message.strip_prefix("Failed to load extended configuration `")?.split_once('`')?.0;
        Some(PathBuf::from(path))
    });
    match extended {
        Some(path) if !path.starts_with(project_root) => anyhow!("`{}` is not valid configuration", path.display()),
        _ => error,
    }
}

// The paths a run lints, canonical, so a file under them is one the run may read and write.
fn linted_roots(files: &[PathBuf]) -> Vec<PathBuf> {
    files.iter().filter_map(|file| fs::canonicalize(file).ok()).collect()
}

// Whether a symlink points at a file outside the paths the run lints, which the run neither reads nor writes.
fn escapes_roots(path: &Path, roots: &[PathBuf]) -> bool {
    path.is_symlink()
        && path.is_file()
        && fs::canonicalize(path).is_ok_and(|target| !roots.iter().any(|root| target.starts_with(root)))
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
    let target_version = FileSettings::new(&settings, resolver.resolve(path), path).from_ruff.target_version;
    let cache_dir = cache::directory(&resolved.cache_dir);
    write_settings(writer, &settings, target_version, &cache_dir)?;
    Ok(())
}

// `ruff check --show-settings`'s layout, with simplint's settings and the Python version Ruff gives the file.
fn write_settings(
    writer: &mut dyn Write,
    settings: &Settings,
    target_version: PythonVersion,
    cache_dir: &Path,
) -> std::io::Result<()> {
    let rules = |set: RuleSet| set.iter().map(|rule| format!("{} ({})", rule.name(), rule.code())).collect_vec();
    let quoted = |items: &mut dyn Iterator<Item = &String>| items.map(|item| format!("{item:?}")).collect_vec();
    let Settings { root, metadata, run, knobs, api, docstring_style, .. } = settings;
    let [enabled, fixable, forced_safe, forced_unsafe] = settings.rule_sets();
    writeln!(writer, "\n# General Settings")?;
    writeln!(writer, "cache_dir = \"{}\"", cache_dir.display())?;
    writeln!(writer, "fix = {}", run.fix.unwrap_or_default())?;
    writeln!(writer, "fix_only = {}", run.fix_only.unwrap_or_default())?;
    writeln!(writer, "output_format = {}", run.output_format.unwrap_or_default())?;
    writeln!(writer, "show_fixes = {}", run.show_fixes.unwrap_or_default())?;
    writeln!(writer, "unsafe_fixes = {}", run.unsafe_fixes())?;
    writeln!(writer, "\n# Linter Settings")?;
    writeln!(writer, "linter.project_root = \"{}\"", root.display())?;
    writeln!(writer, "linter.target_version = {target_version}")?;
    write_list(writer, "linter.rules.enabled", &rules(enabled))?;
    write_list(writer, "linter.rules.should_fix", &rules(fixable))?;
    let per_file_ignores = settings.per_file_ignores().map(|(pattern, set)| format!("{pattern:?} = {:?}", rules(set)));
    write_list(writer, "linter.per_file_ignores", &per_file_ignores.collect_vec())?;
    write_list(writer, "linter.safety_table.forced_safe", &rules(forced_safe))?;
    write_list(writer, "linter.safety_table.forced_unsafe", &rules(forced_unsafe))?;
    write_list(writer, "linter.dependencies", &quoted(&mut metadata.dependencies.iter()))?;
    write_list(writer, "linter.entry_points", &quoted(&mut metadata.entry_points.iter()))?;
    writeln!(writer, "linter.max_paragraph_words = {}", knobs.max_paragraph_words)?;
    write_list(writer, "linter.naming.keep_names", &quoted(&mut knobs.keep_names.iter()))?;
    write_list(writer, "linter.imports.module_attributes", &quoted(&mut knobs.module_attributes.iter()))?;
    writeln!(writer, "linter.logic.max_helper_statements = {}", knobs.max_helper_statements)?;
    writeln!(writer, "linter.logic.max_caller_statements = {}", knobs.max_caller_statements)?;
    writeln!(writer, "linter.project_kind = {}", api.kind)?;
    match &api.packages {
        Some(packages) => write_list(writer, "linter.api_packages", &quoted(&mut packages.iter()))?,
        None => writeln!(writer, "linter.api_packages = every module outside docs and tests")?,
    }
    let modules = api.modules.iter().map(|module| format!("{:?}", module.join("."))).collect_vec();
    write_list(writer, "linter.api_modules", &modules)?;
    match docstring_style {
        Some(style) => writeln!(writer, "linter.docstring_style = {style}"),
        None => writeln!(writer, "linter.docstring_style = none"),
    }
}

fn write_list(writer: &mut dyn Write, name: &str, items: &[String]) -> std::io::Result<()> {
    if items.is_empty() {
        return writeln!(writer, "{name} = []");
    }
    writeln!(writer, "{name} = [")?;
    for item in items {
        writeln!(writer, "\t{item},")?;
    }
    writeln!(writer, "]")
}

// The files simplint checks, from Ruff's resolver, so the walk, `.gitignore`, the default and configured excludes and
// the per-file target version match `ruff check`. Stubs are not linted, but the names they declare are the API of
// their module. Files Ruff's `lint.exclude` leaves out are not linted either, yet the run still indexes them: a package
// `__init__.py` excluded there still imports and re-exports the names of the modules it covers.
struct Discovered<'a> {
    python: Vec<PathBuf>,
    unlinted: Vec<PathBuf>,
    resolver: Resolver<'a>,
}

fn discover<'a>(files: &[PathBuf], configuration: &'a Resolved) -> Result<Discovered<'a>> {
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
    // A suppression comment changes no file's facts, so one round is final.
    let roots = linted_roots(files);
    let counts = for_each_file(files, resolved, None, None, |DiscoveredFile { path, linter, .. }| {
        let source = match std::fs::read_to_string(path) {
            Ok(source) => source,
            Err(error) => {
                error!("Failed to extract source from {}: {error}", path.display());
                return 0;
            }
        };
        let (count, output) = linter.add_suppressions(path, &source, kind, reason);
        if count > 0
            && let Err(error) = write_source(path, &roots, &source, &output)
        {
            error!("Failed to add suppression to {}: {error}", path.display());
            return 0;
        }
        count
    })?;
    Ok(counts.into_iter().flatten().flatten().sum())
}

// One discovered Python file, with its linter, and its package's cache with its key when the cache is on.
struct DiscoveredFile<'a> {
    path: &'a Path,
    linter: Linter<'a>,
    cache: Option<(&'a Cache, FileKey)>,
}

// A file the run indexes, with what indexing and linting it take, worked out once: its settings and Ruff's with the
// configuration file they come from, its package, and the module it imports as from that package. Ruff sees no package
// around a file of a namespace package, yet Python resolves its relative imports, so such a file is `importable` from
// a search root instead.
struct RunFile<'a> {
    path: &'a Path,
    settings: FileSettings<'a>,
    ruff: &'a ruff_workspace::Settings,
    ruff_config: Option<&'a Path>,
    package: Option<PackageRoot<'a>>,
    module_path: Option<Vec<String>>,
    importable: Option<Vec<String>>,
}

// Ruff's per-file loop over the discovered files, with the import index of the whole run built first, and the caches
// in `cache_dir` read before and written after. With `rewrote` set, the files a round rewrites are indexed and linted
// again, as `lint_rounds` tells. `None` when there is no Python file to check.
fn for_each_file<T: Send>(
    files: &[PathBuf],
    resolved: &Resolved,
    cache_dir: Option<&Path>,
    rewrote: Option<&(dyn Fn(&T) -> bool + Sync)>,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Result<Option<Vec<Vec<T>>>> {
    let start = Instant::now();
    let Discovered { python, unlinted, resolver } = discover(files, resolved)?;
    debug!("Identified files to lint in: {:?}", start.elapsed());
    if python.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
        return Ok(None);
    }
    let paths: Vec<&Path> = python.iter().chain(&unlinted).map(PathBuf::as_path).collect();
    let settings: Vec<Arc<Settings>> =
        paths.iter().map(|path| resolved.settings_resolver.for_file(path)).collect::<Result<_>>()?;
    let packages = resolver.package_roots(&paths);
    let roots = SearchRoots::new(&resolved.pyproject_config.settings, packages.values().flatten().copied());
    let indexed: Vec<RunFile> = paths
        .iter()
        .zip(&settings)
        .map(|(&path, settings)| {
            let (ruff, ruff_config) = resolver.resolve_with_path(path);
            let package = path.parent().and_then(|parent| packages.get(parent)).copied().flatten();
            let module_path = package.and_then(|package| to_module_path(package.path(), path));
            let importable = if module_path.is_none() { roots.module_path(path) } else { None };
            let settings = FileSettings::new(settings, ruff, path);
            RunFile { path, settings, ruff, ruff_config, package, module_path, importable }
        })
        .collect();
    let linted = &indexed[..python.len()];
    let scopes = linted
        .iter()
        .map(|file| Scope {
            path: file.path,
            settings: file.settings,
            ruff: file.ruff,
            ruff_config: file.ruff_config,
            package: file.package,
        })
        .collect_vec();
    conflicts::ensure_compatible(&scopes, resolved.names_rules())?;
    let entry_points: Vec<String> = linted
        .iter()
        .map(|file| file.settings.project)
        .unique_by(|settings| std::ptr::from_ref(*settings))
        .flat_map(|settings| settings.metadata.entry_points.iter().cloned())
        .sorted_unstable()
        .dedup()
        .collect();
    let caches = cache_dir.map(|cache_dir| {
        Caches::open(cache_dir, packages.iter().map(|(parent, package)| package.map_or(*parent, PackageRoot::path)))
    });
    // As in Ruff, a file's cache is its package's, or its directory's outside a package.
    let file_caches: FxHashMap<&Path, (&Cache, FileKey)> = caches.as_ref().map_or_else(FxHashMap::default, |caches| {
        indexed
            .par_iter()
            .filter_map(|file| {
                let root = file.package.map(PackageRoot::path).or_else(|| file.path.parent())?;
                Some((file.path, (caches.get(root)?, FileKey::of(file.path)?)))
            })
            .collect()
    });
    let start = Instant::now();
    let mut facts: FxHashMap<&Path, Cow<FileFacts>> = indexed
        .par_iter()
        .filter_map(|file| Some((file.path, file_facts(file, file_caches.get(file.path).copied())?)))
        .collect();
    let rewrites_calls: FxHashSet<&Path> =
        linted.iter().filter(|file| file.settings.rewrites_dataclass_calls()).map(|file| file.path).collect();
    let roles = module_roles(&indexed, &roots);
    let test_modules: FxHashSet<&Path> =
        indexed.iter().filter(|file| file.settings.is_test_module(file.path)).map(|file| file.path).collect();
    let inputs = IndexInputs {
        entry_points: &entry_points,
        rewrites_calls: &rewrites_calls,
        roles: &roles,
        test_modules: &test_modules,
    };
    let mut imports = ImportResolver::new(&roots)?;
    let project = index_project(&facts, &mut imports, &inputs);
    debug!("Indexed {} files in: {:?}", facts.len(), start.elapsed());
    let start = Instant::now();
    let index = Index { facts: &mut facts, resolver: &mut imports, inputs: &inputs };
    let passes = lint_rounds(linted, index, project, &file_caches, rewrote, work);
    debug!("Checked {:?} files in: {:?}", passes.len(), start.elapsed());
    // The facts borrow from the caches, which writing consumes.
    drop(facts);
    if let Some(caches) = caches {
        caches.persist()?;
    }
    Ok(Some(passes))
}

// What a fix round indexes the run from: each file's facts, where their module names lead, and what else the index
// reads.
struct Index<'i, 'f> {
    facts: &'i mut FxHashMap<&'f Path, Cow<'f, FileFacts>>,
    resolver: &'i mut ImportResolver,
    inputs: &'i IndexInputs<'i>,
}

// Each file's passes, in order. A fix in one file can change what another learns from the index, as folding
// `mock.uuid = 7` into the mock's constructor leaves no assignment to stop a dataclass with a `uuid` field from
// freezing. So with `rewrote` set, the files a round rewrites have their facts collected again, and each file whose
// answers from the index change is linted again, until a round rewrites nothing.
fn lint_rounds<'f, T: Send>(
    files: &'f [RunFile<'f>],
    mut index: Index<'_, 'f>,
    mut project: Project,
    file_caches: &FxHashMap<&Path, (&'f Cache, FileKey)>,
    rewrote: Option<&(dyn Fn(&T) -> bool + Sync)>,
    work: impl Fn(&DiscoveredFile) -> T + Sync,
) -> Vec<Vec<T>> {
    // Two files whose fixes keep changing each other's facts would not settle; past this many rounds, the last
    // reports stand.
    const ROUNDS: usize = 8;
    // A file a fix rewrote has a new key.
    let recache = |path: &Path| Some((file_caches.get(path)?.0, FileKey::of(path)?));
    // Without rewrites nothing indexes again, so the facts and resolved module names go before the files are linted.
    if rewrote.is_none() {
        index.facts.clear();
        index.resolver.release();
    }
    let lint = |file: &RunFile, project: &Project, cache| {
        work(&DiscoveredFile { path: file.path, linter: file_linter(file, project), cache })
    };
    let mut passes: Vec<Vec<T>> =
        files.par_iter().map(|file| vec![lint(file, &project, file_caches.get(file.path).copied())]).collect();
    let Some(rewrote) = rewrote else {
        return passes;
    };
    let mut rewritten: Vec<usize> = (0..passes.len()).filter(|&at| passes[at].iter().any(rewrote)).collect();
    // Each file's words, read when a round first needs them and again once a round rewrites the file.
    let mut words: Vec<Option<FxHashSet<String>>> = vec![None; files.len()];
    for _ in 1..ROUNDS {
        if rewritten.is_empty() {
            break;
        }
        for &at in &rewritten {
            words[at] = None;
        }
        let changed: Vec<&RunFile> = rewritten.iter().map(|&at| &files[at]).collect();
        let collect = |file: &RunFile| file_facts(file, recache(file.path));
        let (next, dependents) = reindex(&mut index, &changed, collect, files, &mut words, &project);
        project = next;
        let relinted: Vec<(usize, T)> =
            dependents.into_par_iter().map(|at| (at, lint(&files[at], &project, recache(files[at].path)))).collect();
        rewritten = relinted.iter().filter(|(_, pass)| rewrote(pass)).map(|(at, _)| *at).collect();
        for (at, pass) in relinted {
            passes[at].push(pass);
        }
    }
    passes
}

// Where each file sits in its library's public API, by the first of the names the file imports as, from its package or
// from a search root, that an API package holds. Unlinted files count too: an `__init__.py` Ruff's `lint.exclude` leaves
// out still re-exports public names.
fn module_roles<'p>(files: &[RunFile<'p>], roots: &SearchRoots) -> FxHashMap<&'p Path, ModuleRole> {
    files
        .iter()
        .map(|file| {
            let settings = file.settings.project;
            let module_paths = file.module_path.iter().cloned().chain(roots.module_paths(file.path)).collect_vec();
            let roles: Vec<ModuleRole> =
                module_paths.iter().map(|module_path| settings.role(file.path, Some(module_path))).collect();
            let role = roles.iter().copied().find(|role| role.public).or_else(|| roles.first().copied());
            (file.path, role.unwrap_or_else(|| settings.role(file.path, None)))
        })
        .collect()
}

// A file's import facts: those its cache holds while the file is unchanged, and otherwise freshly collected and stored
// there.
fn file_facts<'c>(file: &RunFile, cache: Option<(&'c Cache, FileKey)>) -> Option<Cow<'c, FileFacts>> {
    if let Some(facts) = cache.and_then(|(cache, key)| cache.facts(file.path, key)) {
        return Some(Cow::Borrowed(facts));
    }
    let facts = FileFacts::collect(file.path, file.module_path.as_deref(), file.importable.as_deref())?;
    if let Some((cache, key)) = cache {
        cache.set_facts(file.path, key, facts.clone());
    }
    Some(Cow::Owned(facts))
}

// The index once the `rewritten` files have their facts collected and their module names resolved again, and the
// positions of the files whose answers from it differ from those of `project`. Each file's `words` are read the first
// time a changed name needs them.
fn reindex<'f>(
    index: &mut Index<'_, 'f>,
    rewritten: &[&RunFile<'f>],
    collect: impl Fn(&RunFile) -> Option<Cow<'f, FileFacts>> + Sync,
    files: &[RunFile],
    words: &mut [Option<FxHashSet<String>>],
    project: &Project,
) -> (Project, Vec<usize>) {
    // A fix never leaves a file that does not parse, so a rewritten file keeps having facts.
    let fresh: Vec<(&Path, FileFacts)> =
        rewritten.par_iter().filter_map(|&file| Some((file.path, collect(file)?.into_owned()))).collect();
    for file in rewritten {
        index.resolver.forget(file.path);
    }
    index.facts.extend(fresh.into_iter().map(|(path, file)| (path, Cow::Owned(file))));
    let next = index_project(index.facts, index.resolver, index.inputs);
    let changed = next.changed_names(project);
    if !changed.is_empty() {
        words.par_iter_mut().zip(files).filter(|(known, _)| known.is_none()).for_each(|(known, file)| {
            *known = Some(words_of(file.path));
        });
    }
    let dependents = (0..files.len())
        .into_par_iter()
        .filter(|&at| {
            let path = files[at].path;
            let mentions = || words[at].as_ref().is_some_and(|held| changed.iter().any(|name| held.contains(*name)));
            next.file(path) != project.file(path) || mentions()
        })
        .collect();
    (next, dependents)
}

fn index_project(
    facts: &FxHashMap<&Path, Cow<FileFacts>>,
    resolver: &mut ImportResolver,
    inputs: &IndexInputs,
) -> Project {
    let indexed: Vec<(&Path, &FileFacts)> = facts.iter().map(|(path, facts)| (*path, facts.as_ref())).collect();
    Project::index(&indexed, resolver, inputs)
}

// The names a file holds, to tell whether it names any of the attribute or base class names whose answers changed; the
// rules ask about names the file itself holds, such as its dataclass fields, so a file without one gets the same
// answers.
fn words_of(path: &Path) -> FxHashSet<String> {
    let source = std::fs::read_to_string(path).unwrap_or_default();
    source.split(|char: char| !(char.is_alphanumeric() || char == '_')).map(str::to_owned).collect()
}

// A file's linter, from simplint's settings and Ruff's for it and the package it sits in.
fn file_linter<'a>(file: &RunFile<'a>, project: &'a Project) -> Linter<'a> {
    Linter::new(file.ruff, file.settings, project.file(file.path), file.package, file.module_path.clone())
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
    let file_settings = FileSettings::new(&settings, ruff, &absolute);
    let scope = Scope {
        path: &absolute,
        settings: file_settings,
        ruff,
        ruff_config: resolved.pyproject_config.path.as_deref(),
        package,
    };
    conflicts::ensure_compatible(&[scope], resolved.names_rules())?;
    let roots = SearchRoots::new(ruff, package);
    let module_path =
        package.and_then(|package| to_module_path(package.path(), &absolute)).or_else(|| roots.module_path(&absolute));
    let roles = FxHashMap::from_iter([(absolute.as_path(), settings.role(&absolute, module_path.as_deref()))]);
    let rewrites_calls = FxHashSet::default();
    let test_modules = FxHashSet::default();
    let inputs = IndexInputs {
        entry_points: &settings.metadata.entry_points,
        rewrites_calls: &rewrites_calls,
        roles: &roles,
        test_modules: &test_modules,
    };
    let project = Project::index(&[], &mut ImportResolver::new(&roots)?, &inputs);
    let package_module = package.and_then(|package| to_module_path(package.path(), &absolute));
    let linter = Linter::new(ruff, file_settings, project.file(&absolute), package, package_module);
    lint_stdin(&path, &source, &linter, mode)
}

pub(super) fn check_files(files: &[PathBuf], resolved: &Resolved, mode: &Mode, use_cache: bool) -> Result<Diagnostics> {
    let cache_dir = use_cache.then_some(resolved.cache_dir.as_path());
    let fingerprints = Fingerprints::default();
    // Only applied fixes reach the disk, where other files' facts come from.
    let rewrote = |diagnostics: &Diagnostics| !diagnostics.fixed.is_empty();
    let results = for_each_file(files, resolved, cache_dir, mode.fix.is_apply().then_some(&rewrote), |file| {
        let lint = |settled| lint_path(file.path, &file.linter, mode, settled);
        let Some((cache, key)) = file.cache else {
            return lint(None).0;
        };
        cache.lint_or_reuse(file.path, key, fingerprints.of(&file.linter, mode), lint)
    })?;
    // A later pass over a file replaces its reports, while the fixes of every pass count.
    let file_results = results.into_iter().flatten().filter_map(|passes| {
        passes.into_iter().reduce(|mut earlier, later| {
            earlier.inner = later.inner;
            earlier.fixed += later.fixed;
            earlier
        })
    });
    Ok(file_results.fold(Diagnostics::default(), |left, right| left + right))
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
