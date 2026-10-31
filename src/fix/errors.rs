use std::path::Path;

use anyhow::{Error, Result, anyhow, bail};
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

pub fn broken_syntax(path: &Path, transformed: &str, fixed: &FixTable) -> Error {
    report_broken_syntax(path, transformed, fixed, SIMPLINT);
    anyhow!("Fix introduced a syntax error")
}

// Ruff's `lint_fix` reports the same bugs in its own fixes.
pub fn ruff_broken_syntax(path: &Path, transformed: &str, fixed: &FixTable) -> Error {
    report_broken_syntax(path, transformed, fixed, RUFF);
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

fn report_broken_syntax(path: &Path, transformed: &str, fixed: &FixTable, tool: Tool) {
    let parsed = parse_unchecked_source(transformed, PySourceType::Python);
    let error = &parsed.errors()[0];
    let rules = codes(fixed.iter().map(|(id, code, _)| code.map_or_else(|| id.as_str(), |code| code.as_str())));
    if cfg!(debug_assertions) {
        eprintln!(
            "{}{} Fix introduced a syntax error in `{}` with rule codes {rules}: {error}\n---\n{transformed}\n---",
            "error".red().bold(),
            ":".bold(),
            relativize_path(path),
        );
    } else {
        report_bug("Fix introduced a syntax error. Reverting all changes.", "Fix%20error", path, &rules, tool);
    }
}

pub fn ensure_converging(path: &Path, iterations: usize, diagnostics: &[Diagnostic]) -> Result<()> {
    if iterations < MAX_ITERATIONS {
        return Ok(());
    }
    let rules = codes(diagnostics.iter().map(Diagnostic::secondary_code_or_id));
    report_bug(
        &format!("Failed to converge after {MAX_ITERATIONS} iterations."),
        "Infinite%20loop",
        path,
        &rules,
        SIMPLINT,
    );
    bail!("Failed to converge")
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
