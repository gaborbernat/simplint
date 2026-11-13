// The `simplint check` flow: the settings and file listings it prints, and the exit status.
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
use crate::cli::write_flushed;
use crate::lint::registry::RuleSet;
use crate::project::{Ending, cache};
use crate::run::{
    CacheUse, Diagnostics, Discovered, Mode, Resolved, add_suppressions, check_files, check_stdin, discover,
    linted_roots, warn_missing,
};
use crate::settings::{FileSettings, Settings};

pub(crate) fn check(cli: &CheckCommand, global_options: &GlobalConfigArgs) -> Result<ExitStatus> {
    let resolved = Resolved::new(cli, global_options)?;
    let is_stdin = reads_stdin(cli);
    let files = if cli.files.is_empty() || is_stdin { vec![PathBuf::from(".")] } else { cli.files.clone() };
    let mut writer: Box<dyn Write> = match &cli.output_file {
        Some(path) if !cli.watch => {
            colored::control::set_override(false);
            path.parent().map(std::fs::create_dir_all).transpose()?;
            Box::new(BufWriter::new(std::fs::File::create(path)?))
        }
        _ => Box::new(BufWriter::new(std::io::stdout())),
    };
    if cli.show_settings || cli.show_files {
        let text = if cli.show_settings { show_settings(&files, &resolved)? } else { show_files(&files, &resolved)? };
        write_flushed(&mut writer, [text])?;
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
    let log_level = global_options.log_level();
    if let Some((reason, kind)) = cli.suppression() {
        let flag = suppression_flag(kind);
        if is_stdin {
            bail!("{flag} cannot be used with standard input");
        }
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
            eprintln!("Added {modifications} {noun}{}.", if modifications == 1 { "" } else { "s" });
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
    if cli.watch {
        let usage = CacheUse::new(cli.no_cache, &mode, None);
        return watch(cli, global_options, &files, resolved, &usage, &printer, writer);
    }
    let mut diagnostics = if is_stdin {
        check_standard_input(cli, &resolved, &mode)?
    } else {
        let usage = CacheUse::new(cli.no_cache, &mode, printer.full_display(&resolved.cwd));
        check_files(&files, &resolved, &mode, &usage, Ending::Exit)?
    };
    diagnostics.inner.sort_by(Diagnostic::ruff_start_ordering);
    // Files are linted in parallel, so their diffs wait here to print in the order the report sorts files.
    diagnostics.diffs.sort_by(|left, right| left.0.cmp(&right.0));
    write_flushed(&mut std::io::stdout().lock(), diagnostics.diffs.iter().map(|(_, diff)| diff))?;
    // With fixed source going to standard output, the report goes to standard error.
    if is_stdin && !mode.fix.is_generate() && cli.output_file.is_none() {
        writer = Box::new(std::io::stderr());
    }
    if cli.statistics {
        printer.write_statistics(&diagnostics, &mut writer)?;
    } else {
        printer.write_once(&diagnostics, &mut writer)?;
    }
    let status = exit_status(cli, &diagnostics, fix_only);
    // The output is out, so the run's results go to the exit rather than being freed one by one.
    Ending::Exit.end(diagnostics);
    Ok(status)
}

// Ruff's `is_stdin`, so one command line reads the same source under both tools.
fn reads_stdin(cli: &CheckCommand) -> bool {
    if cli.stdin_filename.is_none() {
        return matches!(cli.files.as_slice(), [file] if file == Path::new("-"));
    }
    if let Some(file) = cli.files.iter().find(|file| file.as_path() != Path::new("-")) {
        warn_user_once!("Ignoring file {} in favor of standard input.", file.display());
    }
    true
}

fn show_settings(files: &[PathBuf], resolved: &Resolved) -> Result<String> {
    let Discovered { python, resolver, missing, .. } = discover(files, resolved)?;
    warn_missing(&missing);
    let Some(path) = python.first() else {
        bail!("No files found under the given path");
    };
    let settings = resolved.settings_resolver.for_file(path)?;
    let settings_path = settings
        .path
        .as_ref()
        .map(|settings_path| format!("Settings path: {:?}\n", settings_path.display().to_string()));
    let target_version = FileSettings::new(&settings, resolver.resolve(path), path).from_ruff.target_version;
    let cache_dir = resolved.cache_dir.as_deref().map(cache::directory);
    Ok(format!(
        "Resolved settings for: {:?}\n{}{}",
        path.display().to_string(),
        settings_path.unwrap_or_default(),
        settings_text(&settings, target_version, cache_dir.as_deref())
    ))
}

// `ruff check --show-settings`'s layout, with simplint's settings and the Python version Ruff gives the file.
fn settings_text(settings: &Settings, target_version: PythonVersion, cache_dir: Option<&Path>) -> String {
    let rules = |set: RuleSet| set.iter().map(|rule| format!("{} ({})", rule.name(), rule.code())).collect_vec();
    let quoted = |items: &mut dyn Iterator<Item = &String>| items.map(|item| format!("{item:?}")).collect_vec();
    let Settings { root, metadata, run, knobs, api, docstring_style, .. } = settings;
    let [enabled, fixable, forced_safe, forced_unsafe] = settings.rule_sets();
    let per_file_ignores = settings.per_file_ignores().map(|(pattern, set)| format!("{pattern:?} = {:?}", rules(set)));
    let lines = [
        "\n# General Settings".to_owned(),
        format!("cache_dir = {}", cache_dir.map_or_else(|| "none".to_owned(), |dir| format!("\"{}\"", dir.display()))),
        format!("fix = {}", run.fix.unwrap_or_default()),
        format!("fix_only = {}", run.fix_only.unwrap_or_default()),
        format!("output_format = {}", run.output_format.unwrap_or_default()),
        format!("show_fixes = {}", run.show_fixes.unwrap_or_default()),
        format!("unsafe_fixes = {}", run.unsafe_fixes()),
        "\n# Linter Settings".to_owned(),
        format!("linter.project_root = \"{}\"", root.display()),
        format!("linter.target_version = {target_version}"),
        list("linter.rules.enabled", &rules(enabled)),
        list("linter.rules.should_fix", &rules(fixable)),
        list("linter.per_file_ignores", &per_file_ignores.collect_vec()),
        list("linter.safety_table.forced_safe", &rules(forced_safe)),
        list("linter.safety_table.forced_unsafe", &rules(forced_unsafe)),
        list("linter.dependencies", &quoted(&mut metadata.dependencies.iter())),
        list("linter.declared_dependencies", &quoted(&mut metadata.declared.iter())),
        list("linter.entry_points", &quoted(&mut metadata.entry_points.iter())),
        format!("linter.max_paragraph_words = {}", knobs.max_paragraph_words),
        list("linter.naming.keep_names", &quoted(&mut knobs.keep_names.iter())),
        list("linter.naming.meaningless_names", &quoted(&mut knobs.meaningless_names.iter())),
        list("linter.naming.catch_all_modules", &quoted(&mut knobs.catch_all_modules.iter())),
        list("linter.imports.module_attributes", &quoted(&mut knobs.module_attributes.iter())),
        list("linter.imports.module_attribute_scope", &quoted(&mut knobs.module_attribute_scope.iter())),
        format!("linter.logic.max_helper_statements = {}", knobs.max_helper_statements),
        format!("linter.logic.max_caller_statements = {}", knobs.max_caller_statements),
        format!("linter.logic.max_tuple_elements = {}", knobs.max_tuple_elements),
        format!("linter.logic.max_cognitive_complexity = {}", knobs.max_cognitive_complexity),
        format!("linter.logic.min_duplicate_statements = {}", knobs.min_duplicate_statements),
        format!("linter.logic.max_expression_complexity = {}", knobs.max_expression_complexity),
        format!("linter.classes.max_instance_attributes = {}", knobs.max_instance_attributes),
        format!("linter.project_kind = {}", api.kind),
        match &api.packages {
            Some(packages) => list("linter.api_packages", &quoted(&mut packages.iter())),
            None => "linter.api_packages = every module outside docs and tests".to_owned(),
        },
        format!(
            "linter.docstring_style = {}",
            docstring_style.map_or_else(|| "none".to_owned(), |style| style.to_string())
        ),
    ];
    lines.iter().format_with("", |line, f| f(&format_args!("{line}\n"))).to_string()
}

fn list(name: &str, items: &[String]) -> String {
    if items.is_empty() {
        return format!("{name} = []");
    }
    format!("{name} = [\n{}]", items.iter().format_with("", |item, f| f(&format_args!("\t{item},\n"))))
}

fn show_files(files: &[PathBuf], resolved: &Resolved) -> Result<String> {
    let discovered = discover(files, resolved)?;
    warn_missing(&discovered.missing);
    if discovered.python.is_empty() {
        warn_user_once!("No Python files found under the given path(s)");
    }
    Ok(discovered.python.iter().format_with("", |path, f| f(&format_args!("{}\n", path.to_string_lossy()))).to_string())
}

// The source comes from standard input, which is linted as the file it stands for.
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
