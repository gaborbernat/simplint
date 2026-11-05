use std::path::Path;

use ruff_macros::CacheKey;
use ruff_python_ast::PythonVersion;
use ruff_workspace::resolver::match_exclusion;

use crate::lint::registry::RuleSet;
use crate::rules::classes::dataclass_kw_only;
use crate::settings::Settings;

// The settings a file is linted with: its project's, the rules left on for it, and what Ruff decides for it.
#[derive(Clone, Copy)]
pub struct FileSettings<'a> {
    pub project: &'a Settings,
    pub from_ruff: FromRuff,
    pub rules: RuleSet,
}

impl<'a> FileSettings<'a> {
    pub fn new(project: &'a Settings, ruff: &ruff_workspace::Settings, path: &Path) -> Self {
        let from_ruff = FromRuff::of(ruff, path);
        Self { project, from_ruff, rules: project.rules_for(path).for_target(from_ruff.target_version) }
    }

    pub fn rewrites_dataclass_calls(&self) -> bool {
        dataclass_kw_only::rewrites_calls(self)
    }

    pub fn is_test_module(&self, path: &Path) -> bool {
        self.project.is_test_module(path)
    }
}

// What Ruff's settings decide for a file: what simplint's rules follow, and how Ruff parses and formats it.
#[derive(Clone, Copy, CacheKey)]
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
        }
    }
}
