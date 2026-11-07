use std::path::Path;
use std::sync::Arc;

use ruff_linter::registry::{Rule as RuffRule, RuleSet as RuffRuleSet};
use ruff_linter::rules::pep8_naming::settings::IgnoreNames;
use ruff_macros::CacheKey;
use ruff_python_ast::PythonVersion;
use ruff_workspace::resolver::match_exclusion;

use crate::lint::registry::RuleSet;
use crate::rules::classes::dataclass_kw_only;
use crate::settings::Settings;

// The settings a file is linted with: its project's, the rules left on for it, and what Ruff decides for it.
#[derive(Clone)]
pub struct FileSettings<'a> {
    pub project: &'a Settings,
    pub from_ruff: FromRuff,
    pub rules: RuleSet,
}

impl<'a> FileSettings<'a> {
    pub fn new(project: &'a Settings, ruff: &ruff_workspace::Settings, path: &Path) -> Self {
        let from_ruff = FromRuff::of(ruff, path);
        let rules = project.rules_for(path).for_target(from_ruff.target_version);
        Self { project, from_ruff, rules }
    }

    pub fn rewrites_dataclass_calls(&self) -> bool {
        dataclass_kw_only::rewrites_calls(self)
    }

    pub fn is_test_module(&self, path: &Path) -> bool {
        self.project.is_test_module(path)
    }
}

// What Ruff's settings decide for a file: what simplint's rules follow, and how Ruff parses and formats it.
#[derive(Clone, CacheKey)]
#[expect(clippy::struct_excessive_bools, reason = "each flag is a Ruff setting that decides one rule's overlap")]
pub struct FromRuff {
    pub target_version: PythonVersion,
    pub magic_trailing_comma: bool,
    // The version Ruff parses the file as: the latest when the project names none, unlike `target_version`.
    pub parser_version: PythonVersion,
    // The line and indent widths `ruff format` gives the file, and whether its settings leave the file out.
    pub line_width: usize,
    pub indent_width: usize,
    pub formatted: bool,
    // The longest comment line Ruff accepts: its line length limit, or its doc line limit when that is shorter.
    pub comment_width: usize,
    // Whether Ruff's preview mode is on, which changes the reports of Ruff rules that simplint's rules work around.
    pub preview: bool,
    // Whether Ruff's `strictly-empty-init-modules` setting bans all code in an `__init__.py`, as RUF067 reads it.
    pub strictly_empty_init_modules: bool,
    // The names pep8-naming leaves alone, which Ruff's N816 skips.
    pub ignored_names: Arc<IgnoreNames>,
    // Whether Ruff's ISC003 leaves a concatenation across lines alone, and whether its ANN401 skips `*args` and
    // `**kwargs`, which decide what the rules that Ruff's checks overlap follow.
    pub allow_multiline: bool,
    pub allow_star_arg_any: bool,
    // The loggers beside the `logging` module that Ruff's logging rules look at, as its G003 does.
    pub logger_objects: Vec<String>,
    // The rules Ruff's settings enable, which decide whether a rule that Ruff covers is left to it. Per-file ignores
    // are not applied: a file that silences a Ruff rule is the project's choice, as a `noqa` for it is.
    ruff_rules: RuffRuleSet,
}

impl FromRuff {
    fn of(settings: &ruff_workspace::Settings, path: &Path) -> Self {
        let target_version = settings.linter.resolve_target_version(path);
        let formatter = &settings.formatter;
        let pycodestyle = &settings.linter.pycodestyle;
        Self {
            target_version: target_version.linter_version(),
            magic_trailing_comma: !formatter.magic_trailing_comma.is_ignore(),
            parser_version: target_version.parser_version(),
            line_width: usize::from(formatter.line_width.value()),
            indent_width: formatter.indent_width.value() as usize,
            formatted: !match_exclusion(path, path.file_name().unwrap_or_default(), &formatter.exclude),
            comment_width: usize::from(pycodestyle.max_line_length.value())
                .min(pycodestyle.max_doc_length.map_or(usize::MAX, |length| usize::from(length.value()))),
            preview: settings.linter.preview.is_enabled(),
            strictly_empty_init_modules: settings.linter.ruff.strictly_empty_init_modules,
            ignored_names: Arc::new(settings.linter.pep8_naming.ignore_names.clone()),
            allow_multiline: settings.linter.flake8_implicit_str_concat.allow_multiline,
            allow_star_arg_any: settings.linter.flake8_annotations.allow_star_arg_any,
            logger_objects: settings.linter.logger_objects.clone(),
            ruff_rules: settings.linter.rules.iter_enabled().collect(),
        }
    }

    // Whether Ruff runs `rule` in this project, so a rule that Ruff covers leaves the case to it.
    pub fn ruff_runs(&self, rule: RuffRule) -> bool {
        self.ruff_rules.contains(rule)
    }
}
