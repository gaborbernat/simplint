// `simplint check`: the CLI flow, the settings and file listings it prints, and the exit status. The run itself lives in
// `run`.
use std::io::{BufWriter, Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use itertools::Itertools;
use ruff::ExitStatus;
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::logging::LogLevel;
use ruff_linter::settings::flags;
use ruff_linter::{SuppressionKind, warn_user, warn_user_once};
use ruff_python_ast::PythonVersion;

use crate::cli::args::{CheckCommand, GlobalConfigArgs, suppression_flag};
use crate::cli::printer::{Flags, Printer};
use crate::cli::watch::watch;
use crate::lint::registry::RuleSet;
use crate::project::cache;
use crate::run::{
    Diagnostics, Discovered, Mode, Resolved, add_suppressions, check_files, check_stdin, discover, linted_roots,
};
use crate::settings::{FileSettings, Settings};

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
    let mut diagnostics = if is_stdin {
        check_standard_input(cli, &resolved, &mode)?
    } else {
        check_files(&files, &resolved, &mode, use_cache)?
    };
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

// The source comes from standard input, which is linted as the file it stands for, and whatever the fixes or the diff
// write goes to standard output.
fn check_standard_input(cli: &CheckCommand, resolved: &Resolved, mode: &Mode) -> Result<Diagnostics> {
    let mut source = String::new();
    std::io::stdin().read_to_string(&mut source)?;
    let path = cli.stdin_filename.clone().unwrap_or_else(|| PathBuf::from("-"));
    let stdin = check_stdin(&path, &source, resolved, mode)?;
    if let Some(output) = stdin.output {
        std::io::stdout().lock().write_all(output.as_bytes())?;
    }
    Ok(stdin.diagnostics)
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
