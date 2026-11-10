use std::fmt;
use std::path::Path;

use anyhow::{Error, anyhow};
use colored::Colorize;
use itertools::Itertools;
use ruff_db::diagnostic::Diagnostic;
use ruff_linter::fs::relativize_path;
use ruff_linter::linter::FixTable;
use ruff_python_ast::PySourceType;
use ruff_python_parser::parse_unchecked_source;

// Only a buggy fix can trip these, so coverage excludes this file; they keep a bug from corrupting user files and
// report it the way Ruff does.

const MAX_ITERATIONS: usize = 100;

// A bug one of simplint's fixes hit. The fix loop drops a withdrawal candidate that hits one, so it is reported only
// once the loop gives up on the file.
#[derive(Debug)]
pub enum FixBug {
    BrokenSyntax { transformed: String, rules: String, error: String },
    NotConverging { rules: String },
}

impl FixBug {
    pub fn report(&self, path: &Path) {
        match self {
            Self::BrokenSyntax { transformed, rules, error } => {
                report_broken_syntax(path, transformed, rules, error, SIMPLINT);
            }
            Self::NotConverging { rules } => report_bug(
                &format!("Failed to converge after {MAX_ITERATIONS} iterations."),
                "Infinite%20loop",
                path,
                rules,
                SIMPLINT,
            ),
        }
    }
}

impl fmt::Display for FixBug {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::BrokenSyntax { .. } => "Fix introduced a syntax error",
            Self::NotConverging { .. } => "Failed to converge",
        })
    }
}

impl std::error::Error for FixBug {}

// Fails with the message of the syntax error a fix introduced, if `introduced` names one.
pub fn ensure_syntax(transformed: &str, fixed: &FixTable, introduced: Result<(), String>) -> Result<(), FixBug> {
    introduced.map_err(|error| FixBug::BrokenSyntax {
        transformed: transformed.to_owned(),
        rules: fixed_codes(fixed),
        error,
    })
}

pub fn ensure_converging(iterations: usize, diagnostics: &[Diagnostic]) -> Result<(), FixBug> {
    if iterations < MAX_ITERATIONS {
        return Ok(());
    }
    Err(FixBug::NotConverging { rules: codes(diagnostics.iter().map(Diagnostic::secondary_code_or_id)) })
}

// Ruff's `lint_fix` reports the same bugs in its own fixes.
pub fn ruff_broken_syntax(path: &Path, transformed: &str, fixed: &FixTable) -> Error {
    let parsed = parse_unchecked_source(transformed, PySourceType::Python);
    report_broken_syntax(path, transformed, &fixed_codes(fixed), &parsed.errors()[0].to_string(), RUFF);
    anyhow!("Fix introduced a syntax error")
}

// Whether Ruff's fixes went on for too many passes, reported as Ruff's `lint_fix` does.
pub fn ruff_failed_to_converge(path: &Path, iterations: usize, transformed: &str, diagnostics: &[Diagnostic]) -> bool {
    if iterations < MAX_ITERATIONS {
        return false;
    }
    let rules = codes(diagnostics.iter().map(Diagnostic::secondary_code_or_id));
    if cfg!(debug_assertions) {
        eprintln!(
            "{}{} Failed to converge after {MAX_ITERATIONS} iterations in `{}` with rule codes {rules}:---\n{transformed}\n---",
            "debug error".red().bold(),
            ":".bold(),
            relativize_path(path),
        );
    } else {
        report_bug(
            &format!("Failed to converge after {MAX_ITERATIONS} iterations."),
            "Infinite%20loop",
            path,
            &rules,
            RUFF,
        );
    }
    true
}

// Who to report a bug to: the project's name and its GitHub repository.
type Tool = (&'static str, &'static str);

const SIMPLINT: Tool = ("simplint", "gaborbernat/simplint");
const RUFF: Tool = ("Ruff", "astral-sh/ruff");

fn report_broken_syntax(path: &Path, transformed: &str, rules: &str, error: &str, tool: Tool) {
    if cfg!(debug_assertions) {
        eprintln!(
            "{}{} Fix introduced a syntax error in `{}` with rule codes {rules}: {error}\n---\n{transformed}\n---",
            "error".red().bold(),
            ":".bold(),
            relativize_path(path),
        );
    } else {
        report_bug("Fix introduced a syntax error. Reverting all changes.", "Fix%20error", path, rules, tool);
    }
}

fn fixed_codes(fixed: &FixTable) -> String {
    codes(fixed.iter().map(|(id, code, _)| code.map_or_else(|| id.as_str(), |code| code.as_str())))
}

fn codes<'a>(codes: impl IntoIterator<Item = &'a str>) -> String {
    codes.into_iter().sorted_unstable().dedup().join(", ")
}

fn report_bug(problem: &str, issue_title: &str, path: &Path, rules: &str, (name, repository): Tool) {
    eprintln!(
        "\n{}{} {problem}\n\nThis indicates a bug in {name}. If you could open an issue at:\n\n    \
         https://github.com/{repository}/issues/new?title=%5B{issue_title}%5D\n\n...quoting the contents of \
         `{}`, the rule codes {rules}, along with the `pyproject.toml` settings and executed command, we'd be very \
         appreciative!\n",
        "error".red().bold(),
        ":".bold(),
        relativize_path(path),
    );
}
