use std::path::Path;

use anyhow::{Result, anyhow};
use ruff_diagnostics::Applicability;
use ruff_python_parser::parse_module;

use crate::checker::{Checker, Diagnostic};
use crate::guards::{ensure_converging, ensure_parses};
use crate::project::Project;
use crate::registry::RuleSet;
use crate::settings::Settings;
use crate::{fix, noqa};

pub struct Linter<'a> {
    pub settings: &'a Settings,
    pub project: &'a Project,
    pub rules: RuleSet,
}

pub struct FixOutcome {
    pub source: String,
    pub fixed: usize,
    pub diagnostics: Vec<Diagnostic>,
}

impl Linter<'_> {
    pub fn lint(&self, path: &Path, source: &str) -> Result<Vec<Diagnostic>> {
        let parsed =
            parse_module(source).map_err(|error| anyhow!("{}: syntax error: {}", path.display(), error.error))?;
        let checker = Checker::new(path, source, &parsed, self);
        let comments: Vec<_> = checker.comments().collect();
        let mut diagnostics = checker.run();
        noqa::filter(source, comments, &mut diagnostics);
        diagnostics.sort_by_key(|diagnostic| (diagnostic.range.start(), diagnostic.rule));
        Ok(diagnostics)
    }

    pub fn fix(&self, path: &Path, source: &str, applicability: Applicability) -> Result<FixOutcome> {
        let mut current = source.to_owned();
        let mut fixed = 0;
        let mut iteration = 0;
        loop {
            ensure_converging(path, iteration)?;
            iteration += 1;
            let diagnostics = self.lint(path, &current)?;
            let Some((next, applied)) = fix::apply(&current, &diagnostics, applicability) else {
                return Ok(FixOutcome { source: current, fixed, diagnostics });
            };
            ensure_parses(path, &next)?;
            current = next;
            fixed += applied;
        }
    }
}
