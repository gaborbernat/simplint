use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use ruff_python_parser::parse_module;
use ruff_source_file::LineIndex;

#[derive(Parser)]
#[command(version, about = "Lint Python code against the simp house style")]
struct Cli {
    #[arg(required = true)]
    paths: Vec<PathBuf>,
}

fn main() -> Result<ExitCode> {
    let cli = Cli::parse();
    let files = python_files(&cli.paths)?;
    let diagnostics: Vec<String> = files
        .par_iter()
        .map(|path| check(path))
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect();
    for diagnostic in &diagnostics {
        println!("{diagnostic}");
    }
    eprintln!("checked {} files, found {} issues", files.len(), diagnostics.len());
    Ok(if diagnostics.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn python_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut walk = ignore::WalkBuilder::new(&paths[0]);
    for path in &paths[1..] {
        walk.add(path);
    }
    let mut files = Vec::new();
    for entry in walk.build() {
        let entry = entry?;
        if entry.file_type().is_some_and(|kind| kind.is_file())
            && entry.path().extension().is_some_and(|ext| ext == "py" || ext == "pyi")
        {
            files.push(entry.into_path());
        }
    }
    Ok(files)
}

fn check(path: &Path) -> Result<Vec<String>> {
    let source = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    let Err(error) = parse_module(&source) else {
        return Ok(Vec::new());
    };
    let location = LineIndex::from_source_text(&source).line_column(error.location.start(), &source);
    Ok(vec![format!(
        "{}:{}:{}: E999 {}",
        path.display(),
        location.line,
        location.column,
        error.error
    )])
}
