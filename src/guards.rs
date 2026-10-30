use std::path::Path;

use anyhow::{Result, bail};
use ruff_python_parser::parse_module;

// Only a buggy fix can trip these, so coverage excludes this file; they keep a bug from corrupting user files.

pub fn ensure_converging(path: &Path, iteration: usize) -> Result<()> {
    const MAX_ITERATIONS: usize = 100;
    if iteration == MAX_ITERATIONS {
        bail!("{}: fixes did not converge after {MAX_ITERATIONS} iterations", path.display());
    }
    Ok(())
}

pub fn ensure_parses(path: &Path, source: &str) -> Result<()> {
    if parse_module(source).is_err() {
        bail!("{}: fixes introduced a syntax error, please report this:\n{source}", path.display());
    }
    Ok(())
}
