use std::collections::BTreeSet;
use std::path::Path;

use ruff_linter::registry::{Rule as RuffRule, RuleSet as RuffRuleSet};
use ruff_linter::rules::flake8_tidy_imports::settings::ApiBan;
use ruff_linter::rules::pep8_naming::settings::IgnoreNames;
use ruff_macros::CacheKey;
use ruff_python_ast::PythonVersion;
use ruff_python_semantic::NameImport;
use ruff_workspace::resolver::match_exclusion;
use rustc_hash::FxHashMap;

use crate::lint::registry::RuleSet;
use crate::settings::Settings;

#[derive(Clone)]
pub struct FileSettings<'a> {
    pub project: &'a Settings,
    pub from_ruff: FromRuff<'a>,
    pub rules: RuleSet,
    pub is_test: bool,
}

impl<'a> FileSettings<'a> {
    pub fn new(project: &'a Settings, ruff: &'a ruff_workspace::Settings, path: &Path) -> Self {
        let from_ruff = FromRuff::of(ruff, path);
        let rules = project.rules_for(path).for_target(from_ruff.target_version);
        Self { project, from_ruff, rules, is_test: project.is_test_module(path) }
    }
}

// What the project's settings hold whole is borrowed, since every file of the project reads the same.
#[derive(Clone, CacheKey)]
#[expect(clippy::struct_excessive_bools, reason = "each flag is a Ruff setting that decides one rule's overlap")]
pub struct FromRuff<'a> {
    pub target_version: PythonVersion,
    pub magic_trailing_comma: bool,
    // The version Ruff parses the file as: the latest when the project names none, unlike `target_version`.
    pub parser_version: PythonVersion,
    pub line_width: usize,
    pub indent_width: usize,
    pub formatted: bool,
    pub comment_width: usize,
    // Whether Ruff's preview mode is on, which changes the reports of Ruff rules that simplint's rules work around.
    pub preview: bool,
    // Whether Ruff's `strictly-empty-init-modules` setting bans all code in an `__init__.py`, as RUF067 reads it.
    pub strictly_empty_init_modules: bool,
    // The names pep8-naming leaves alone, which Ruff's N816 skips.
    pub ignored_names: &'a IgnoreNames,
    // Whether Ruff's ISC003 leaves a concatenation across lines alone, and whether its ANN401 skips `*args` and
    // `**kwargs`, which decide what the rules that Ruff's checks overlap follow.
    pub allow_multiline: bool,
    pub allow_star_arg_any: bool,
    // The loggers beside the `logging` module that Ruff's logging rules look at, as its G003 does.
    pub logger_objects: &'a [String],
    // The calls beyond Ruff's own list that its B008 leaves alone in a parameter default.
    pub extend_immutable_calls: &'a [String],
    // The limits of Ruff's complexity and size rules, which an inlined helper can push a caller past.
    pub max_complexity: usize,
    pub max_branches: usize,
    pub max_statements: usize,
    pub max_returns: usize,
    pub max_args: usize,
    pub max_locals: usize,
    pub max_nested_blocks: usize,
    pub max_statements_in_try: usize,
    // The modules and members Ruff's TID251 bans, by qualified name.
    pub banned_api: &'a FxHashMap<String, ApiBan>,
    // The builtin names Ruff's flake8-builtins rules, as its A004, let a binding shadow.
    pub builtins_ignorelist: &'a [String],
    // The imports Ruff's F401 never reports: those isort requires and those pyflakes allows unused.
    pub required_imports: &'a BTreeSet<NameImport>,
    pub allowed_unused_imports: &'a [String],
    // The decorators whose functions Ruff's pydocstyle rules skip.
    pub ignore_decorators: &'a BTreeSet<String>,
    // The rules Ruff's settings enable, which decide whether a rule that Ruff covers is left to it. Per-file ignores
    // are not applied: a file that silences a Ruff rule is the project's choice, as a `noqa` for it is.
    ruff_rules: RuffRuleSet,
}

impl<'a> FromRuff<'a> {
    fn of(settings: &'a ruff_workspace::Settings, path: &Path) -> Self {
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
            ignored_names: &settings.linter.pep8_naming.ignore_names,
            allow_multiline: settings.linter.flake8_implicit_str_concat.allow_multiline,
            allow_star_arg_any: settings.linter.flake8_annotations.allow_star_arg_any,
            logger_objects: &settings.linter.logger_objects,
            extend_immutable_calls: &settings.linter.flake8_bugbear.extend_immutable_calls,
            max_complexity: settings.linter.mccabe.max_complexity,
            max_branches: settings.linter.pylint.max_branches,
            max_statements: settings.linter.pylint.max_statements,
            max_returns: settings.linter.pylint.max_returns,
            max_args: settings.linter.pylint.max_args,
            max_locals: settings.linter.pylint.max_locals,
            max_nested_blocks: settings.linter.pylint.max_nested_blocks,
            max_statements_in_try: settings.linter.pylint.max_statements_in_try,
            banned_api: &settings.linter.flake8_tidy_imports.banned_api,
            builtins_ignorelist: &settings.linter.flake8_builtins.ignorelist,
            required_imports: &settings.linter.isort.required_imports,
            allowed_unused_imports: &settings.linter.pyflakes.allowed_unused_imports,
            ignore_decorators: &settings.linter.pydocstyle.ignore_decorators,
            ruff_rules: settings.linter.rules.iter_enabled().collect(),
        }
    }

    // Whether Ruff runs `rule` in this project, so a rule that Ruff covers leaves the case to it.
    pub fn ruff_runs(&self, rule: RuffRule) -> bool {
        self.ruff_rules.contains(rule)
    }
}
