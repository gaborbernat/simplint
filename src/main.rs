mod checker;
mod fix;
mod guards;
mod linter;
mod noqa;
mod project;
mod registry;
mod rules;
mod semantic;
mod settings;
#[cfg(test)]
mod snapshot;

use std::collections::HashMap;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Args, Parser, Subcommand};
use rayon::prelude::*;
use ruff_diagnostics::Applicability;
use ruff_source_file::LineIndex;
use similar::TextDiff;

use crate::checker::Diagnostic;
use crate::linter::Linter;
use crate::project::Project;
use crate::registry::{FixAvailability, Rule};
use crate::settings::Settings;

#[derive(Parser)]
#[command(version, about = "Ruff companion linter enforcing the simp house style")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Check(CheckArgs),
    Rules,
}

#[derive(Args)]
struct CheckArgs {
    #[arg(default_value = ".")]
    paths: Vec<PathBuf>,
    #[arg(long)]
    fix: bool,
    #[arg(long)]
    unsafe_fixes: bool,
    #[arg(long, conflicts_with = "fix")]
    diff: bool,
    #[arg(long, value_delimiter = ',')]
    select: Vec<String>,
    #[arg(long, value_delimiter = ',')]
    ignore: Vec<String>,
}

fn main() -> Result<ExitCode> {
    match Cli::parse().command {
        Command::Check(args) => check(&args),
        Command::Rules => {
            for rule in Rule::ALL {
                let fix = match rule.fix_availability() {
                    FixAvailability::Always => " (fix)",
                    FixAvailability::Sometimes => " (fix sometimes)",
                    FixAvailability::None => "",
                };
                println!("{} {}{fix}", rule.code(), rule.name());
            }
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn check(args: &CheckArgs) -> Result<ExitCode> {
    let files = discover(args)?;
    let paths: Vec<PathBuf> = files.iter().map(|(path, _)| path.clone()).collect();
    let mut entry_points: Vec<String> =
        files.iter().flat_map(|(_, settings)| settings.entry_points.iter().cloned()).collect();
    entry_points.sort_unstable();
    entry_points.dedup();
    let project = Project::index(&paths, &entry_points);
    let reports = files
        .par_iter()
        .map(|(path, settings)| {
            report(path, &Linter { settings, project: &project, rules: settings.rules_for(path) }, args)
        })
        .collect::<Result<Vec<_>>>()?;
    let mut remaining = 0;
    let mut fixed = 0;
    let mut fixable = 0;
    let mut hidden = 0;
    for report in &reports {
        print!("{}", report.output);
        remaining += report.diagnostics;
        fixed += report.fixed;
        fixable += report.fixable;
        hidden += report.hidden;
    }
    if fixed > 0 {
        eprintln!("Fixed {fixed} issues.");
    }
    if remaining == 0 {
        if fixed == 0 {
            eprintln!("All checks passed!");
        }
        return Ok(ExitCode::SUCCESS);
    }
    eprintln!("Found {remaining} issues.");
    if fixable > 0 {
        eprintln!("[*] {fixable} fixable with `--fix`.");
    }
    if hidden > 0 {
        eprintln!("{hidden} hidden fixes can be enabled with `--unsafe-fixes`.");
    }
    Ok(ExitCode::FAILURE)
}

fn discover(args: &CheckArgs) -> Result<Vec<(PathBuf, Arc<Settings>)>> {
    let mut cache: HashMap<PathBuf, Arc<Settings>> = HashMap::new();
    let mut walk = ignore::WalkBuilder::new(&args.paths[0]);
    for path in &args.paths[1..] {
        walk.add(path);
    }
    let mut files = Vec::new();
    for entry in walk.build() {
        let entry = entry?;
        let path = entry.path();
        if !entry.file_type().is_some_and(|kind| kind.is_file()) || path.extension().is_none_or(|ext| ext != "py") {
            continue;
        }
        let directory = path.parent().unwrap_or(Path::new(".")).to_path_buf();
        let settings = if let Some(settings) = cache.get(&directory) {
            Arc::clone(settings)
        } else {
            let settings = Arc::new(Settings::discover(path)?.with_rules(&args.select, &args.ignore)?);
            cache.insert(directory, Arc::clone(&settings));
            settings
        };
        if !settings.is_excluded(path) {
            files.push((path.to_path_buf(), settings));
        }
    }
    files.sort_by(|(left, _), (right, _)| left.cmp(right));
    Ok(files)
}

struct Report {
    output: String,
    diagnostics: usize,
    fixed: usize,
    fixable: usize,
    hidden: usize,
}

fn report(path: &Path, linter: &Linter, args: &CheckArgs) -> Result<Report> {
    let source = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let mut output = String::new();
    let (diagnostics, fixed, text) = if args.fix || args.diff {
        let applicability = if args.unsafe_fixes { Applicability::Unsafe } else { Applicability::Safe };
        let outcome = linter.fix(path, &source, applicability)?;
        if args.fix && outcome.fixed > 0 {
            std::fs::write(path, &outcome.source).with_context(|| format!("writing {}", path.display()))?;
        }
        if args.diff && outcome.fixed > 0 {
            let name = path.display().to_string();
            output.push_str(
                &TextDiff::from_lines(&source, &outcome.source).unified_diff().header(&name, &name).to_string(),
            );
        }
        (outcome.diagnostics, outcome.fixed, outcome.source)
    } else {
        (linter.lint(path, &source)?, 0, source)
    };
    if !args.diff {
        let index = LineIndex::from_source_text(&text);
        for diagnostic in &diagnostics {
            let location = index.line_column(diagnostic.range.start(), &text);
            let marker =
                if diagnostic.fix.as_ref().is_some_and(|fix| fix.applies(Applicability::Safe)) { " [*]" } else { "" };
            writeln!(
                output,
                "{}:{}:{}: {}{marker} {}",
                path.display(),
                location.line,
                location.column,
                diagnostic.rule.code(),
                diagnostic.message
            )
            .expect("writing to a String cannot fail");
        }
    }
    let fixable = count_fixes(&diagnostics, Applicability::Safe);
    Ok(Report {
        output,
        diagnostics: diagnostics.len(),
        fixed,
        fixable,
        hidden: count_fixes(&diagnostics, Applicability::Unsafe) - fixable,
    })
}

fn count_fixes(diagnostics: &[Diagnostic], applicability: Applicability) -> usize {
    diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.fix.as_ref().is_some_and(|fix| fix.applies(applicability)))
        .count()
}
