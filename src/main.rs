mod cli;
mod fix;
mod layout;
mod lint;
mod project;
mod rst;
mod rules;
mod run;
mod semantic;
mod settings;
mod syntax;

use std::ffi::OsString;
use std::io::Write;
use std::panic::resume_unwind;
use std::process::ExitCode;
use std::thread;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use colored::Colorize;
use rayon::ThreadPoolBuilder;
use ruff::ExitStatus;
use ruff_linter::logging::LogLevel;

use crate::cli::args::{Cli, Command, TerminalColor};
use crate::cli::check::check;
use crate::cli::subcommands;

// Ruff's allocators, on the same platforms: linting allocates many small objects across threads, which the system
// allocator serves slower.
#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[cfg(all(
    not(target_os = "windows"),
    not(target_os = "openbsd"),
    not(target_os = "aix"),
    not(target_os = "android"),
    any(target_arch = "x86_64", target_arch = "aarch64", target_arch = "powerpc64", target_arch = "riscv64")
))]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> ExitCode {
    let cli = Cli::parse();
    // Deeply nested source, such as a list nested thousands of levels deep, recurses past the default stack of a thread
    // in the parser and in every visitor, so the work and rayon's workers get a larger one.
    let worker = thread::Builder::new()
        .stack_size(STACK_SIZE)
        .spawn(move || {
            ThreadPoolBuilder::new().stack_size(STACK_SIZE).build_global().expect("nothing built the pool before");
            run(cli)
        })
        .expect("the process can start its first thread");
    match worker.join() {
        Ok(Ok(status)) => status.into(),
        Ok(Err(error)) => report_error(&error),
        Err(panic) => resume_unwind(panic),
    }
}

const STACK_SIZE: usize = 64 * 1024 * 1024;

fn run(Cli { command, global_options }: Cli) -> Result<ExitStatus> {
    if let Some(color) = colored_override(global_options.color, std::env::var_os("FORCE_COLOR")) {
        colored::control::set_override(color);
    }
    set_up_logging(global_options.log_level());
    match command {
        Command::Check(args) => return check(&args, &global_options),
        Command::Rule { rule, all, output_format } => {
            if all {
                subcommands::rules(output_format)?;
            }
            if let Some(rule) = rule {
                subcommands::rule(rule, output_format)?;
            }
        }
        Command::Rules { output_format } => subcommands::rule_list(output_format)?,
        Command::Config { option, output_format } => subcommands::config(option.as_deref(), output_format)?,
        Command::Linter { output_format } => subcommands::linter(output_format)?,
        Command::Clean => subcommands::clean(global_options.log_level())?,
        Command::GenerateShellCompletion { shell } => shell.generate(&mut Cli::command(), &mut std::io::stdout()),
        Command::Version { output_format } => subcommands::version(output_format)?,
        Command::CliSchema => subcommands::cli_schema(&Cli::command())?,
        Command::ConfigSchema => subcommands::config_schema()?,
    }
    Ok(ExitStatus::Success)
}

// Ruff's `colored_override`: `--color` wins over a nonempty `FORCE_COLOR`.
fn colored_override(color: Option<TerminalColor>, force_color: Option<OsString>) -> Option<bool> {
    match color {
        Some(TerminalColor::Always) => Some(true),
        Some(TerminalColor::Never) => Some(false),
        Some(TerminalColor::Auto) | None => force_color.map(|force_color| !force_color.is_empty()),
    }
}

// Ruff's `set_up_logging`, without the warnings Ruff gives about its own configuration while simplint resolves it:
// `ruff check` already shows them, and simplint would repeat them on every run.
fn set_up_logging(level: LogLevel) {
    fern::Dispatch::new()
        .format(|out, message, record| match record.level() {
            log::Level::Error => out.finish(format_args!("{}{} {message}", "error".red().bold(), ":".bold())),
            log::Level::Warn => out.finish(format_args!("{}{} {message}", "warning".yellow().bold(), ":".bold())),
            log::Level::Info | log::Level::Debug | log::Level::Trace => out.finish(format_args!(
                "{}[{}][{}] {message}",
                jiff::Zoned::now().strftime("[%Y-%m-%d][%H:%M:%S]"),
                record.target(),
                record.level(),
            )),
        })
        .level(match level {
            LogLevel::Default => log::LevelFilter::Info,
            LogLevel::Verbose => log::LevelFilter::Debug,
            LogLevel::Quiet | LogLevel::Silent => log::LevelFilter::Off,
        })
        .level_for("globset", log::LevelFilter::Warn)
        .level_for("ty_python_semantic", log::LevelFilter::Warn)
        .level_for("salsa", log::LevelFilter::Warn)
        .filter(|metadata| {
            metadata.level() != log::Level::Warn
                || (!metadata.target().starts_with("ruff_workspace")
                    && (metadata.target() != "ruff_linter::noqa" || lint::ruff::directive_warnings_shown()))
        })
        .chain(std::io::stderr())
        .apply()
        .expect("nothing set a logger before");
}

// As `ruff` does: a failed run prints its cause chain and exits 2, while a closed pipe ends the run quietly.
fn report_error(error: &anyhow::Error) -> ExitCode {
    if error.chain().any(|cause| {
        cause.downcast_ref::<std::io::Error>().is_some_and(|io| io.kind() == std::io::ErrorKind::BrokenPipe)
    }) {
        return ExitCode::from(0);
    }
    let mut stderr = std::io::stderr().lock();
    writeln!(stderr, "{}", "simplint failed".red().bold()).ok();
    for cause in error.chain() {
        writeln!(stderr, "  {} {cause}", "Cause:".bold()).ok();
    }
    ExitStatus::Error.into()
}
