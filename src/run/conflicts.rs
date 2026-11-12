use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Result, bail};
use itertools::Itertools;
use rayon::prelude::*;
use ruff_linter::fs::relativize_path;
use ruff_linter::linter::{ParseSource, lint_only};
use ruff_linter::package::PackageRoot;
use ruff_linter::registry::Rule as RuffRule;
use ruff_linter::settings::LinterSettings;
use ruff_linter::settings::flags;
use ruff_linter::settings::rule_table::RuleTable;
use ruff_linter::source_kind::SourceKind;
use ruff_python_ast::{PySourceType, PythonVersion};
use ruff_python_semantic::NameImport;

use crate::lint::registry::Rule;
use crate::settings::{DocstringStyle, FileSettings};

// A file the run is about to lint: simplint's settings for it, Ruff's with the configuration file they come from, and
// the package it sits in, which decides whether Ruff counts its module as public.
pub(super) struct Scope<'a> {
    pub(crate) path: &'a Path,
    pub(crate) settings: &'a FileSettings<'a>,
    pub(crate) ruff: &'a ruff_workspace::Settings,
    pub(super) ruff_config: Option<&'a Path>,
    pub(crate) package: Option<PackageRoot<'a>>,
}

// Fails when a simplint rule and Ruff rules on for the same file work against each other: one side's fix undoes the
// other's, or leaves what the other reports, so no run of `ruff check --fix` and `simplint check --fix` leaves the file
// clean. Per-file ignores can turn either side off for some files, so every file counts, but files that share both
// settings conflict the same way, and the search for each pair of settings stops at its first conflicting file.
pub(super) fn ensure_compatible(scopes: &[Scope], by_name: bool) -> Result<()> {
    let found = find_conflicts(scopes);
    if found.is_empty() {
        return Ok(());
    }
    bail!(
        "simplint and Ruff rules work against each other, so fixing never leaves the code clean:{}",
        describe(found, by_name)
    )
}

fn find_conflicts<'a>(scopes: &'a [Scope]) -> BTreeMap<Conflict<'a>, &'a Path> {
    // Directories without a `pyproject.toml` get settings of their own that name no file, so groups go in file order
    // for the report to name the same first file every run.
    let groups = scopes
        .iter()
        .into_group_map_by(|scope| {
            (std::ptr::from_ref(scope.settings.project) as usize, std::ptr::from_ref(scope.ruff) as usize)
        })
        .into_values()
        .sorted_by_key(|group| group[0].path)
        .collect_vec();
    let mut found: BTreeMap<Conflict, &Path> = BTreeMap::new();
    for (index, clash) in CLASHES.iter().enumerate() {
        for group in &groups {
            let Some(narrowed) = clash.narrowed(&group[0].ruff.linter) else {
                continue;
            };
            let Some((scope, rules)) =
                group.par_iter().find_map_first(|scope| Some(*scope).zip(clash.reported(scope, &narrowed)))
            else {
                continue;
            };
            let conflict = Conflict {
                clash: index,
                simplint_config: scope.settings.project.path.as_deref(),
                ruff_config: scope.ruff_config,
                rules,
            };
            found.entry(conflict).or_insert(scope.path);
        }
    }
    found
}

impl Clash {
    // Ruff's settings with only this clash's rules on, so the probe runs nothing else; `None` when none is on.
    fn narrowed(&self, ruff: &LinterSettings) -> Option<LinterSettings> {
        let enabled = self.ruff.iter().copied().filter(|rule| ruff.rules.enabled(*rule)).collect_vec();
        if enabled.is_empty() {
            return None;
        }
        let mut narrowed = ruff.clone();
        narrowed.rules = RuleTable::empty();
        for rule in enabled {
            narrowed.rules.enable(rule, false);
        }
        Some(narrowed)
    }

    // The positions in `ruff` of the Ruff rules that work against the simplint rule in the file; `None` when either
    // side is off there.
    fn reported(&self, scope: &Scope, narrowed: &LinterSettings) -> Option<Vec<usize>> {
        if !scope.settings.rules.contains(self.simplint) || !(self.acts_on)(scope.settings, narrowed) {
            return None;
        }
        let diagnostics = lint_only(
            scope.path,
            scope.package,
            narrowed,
            flags::Noqa::Disabled,
            &SourceKind::Python { code: self.probe.to_owned(), is_stub: false },
            PySourceType::Python,
            ParseSource::None,
        )
        .diagnostics;
        let rules = self
            .ruff
            .iter()
            .positions(|rule| diagnostics.iter().any(|diagnostic| diagnostic.name() == rule.name().as_str()))
            .collect_vec();
        (!rules.is_empty()).then_some(rules)
    }
}

fn describe(found: BTreeMap<Conflict, &Path>, by_name: bool) -> String {
    let place = |config: Option<&Path>| {
        config.map_or_else(|| "no configuration file".to_owned(), |path| path.display().to_string())
    };
    found
        .into_iter()
        .map(|(Conflict { clash: index, simplint_config, ruff_config, rules }, path)| {
            let clash = &CLASHES[index];
            let rules = rules.into_iter().map(|position| clash.ruff[position]).collect_vec();
            let ruff_codes = rules
                .iter()
                .map(|rule| rule.noqa_code().expect("every Ruff rule in a clash has a code").to_string())
                .collect_vec();
            let names = Names {
                by_name,
                simplint: shown(by_name, clash.simplint.name(), clash.simplint.code()),
                simplint_selector: pick(by_name, clash.simplint.name(), clash.simplint.code()),
                ruff: rules
                    .iter()
                    .zip(&ruff_codes)
                    .map(|(rule, code)| shown(by_name, rule.name().as_str(), code))
                    .join(", "),
                ruff_selectors: rules
                    .iter()
                    .zip(&ruff_codes)
                    .map(|(rule, code)| pick(by_name, rule.name().as_str(), code))
                    .join(", "),
                many: rules.len() > 1,
            };
            format!(
                "\n  - {} and Ruff's {} {}.\n    Why: {}.\n    Fix: {}.\n    simplint settings: {}; Ruff settings: {}; \
                 first file: {}",
                names.simplint,
                names.ruff,
                if clash.undoes_fixes { "undo each other's fixes" } else { "work against each other" },
                (clash.why)(&names),
                (clash.fix)(&names),
                place(simplint_config),
                place(ruff_config),
                relativize_path(path),
            )
        })
        .format("")
        .to_string()
}

// The selector a rule is turned off by: its name or its code.
fn pick(by_name: bool, name: &str, code: &str) -> String {
    if by_name { name.to_owned() } else { code.to_owned() }
}

// A clash found for some files: which one, where each side's settings come from, and where its Ruff rules that
// apply sit in the clash's list.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Conflict<'a> {
    clash: usize,
    simplint_config: Option<&'a Path>,
    ruff_config: Option<&'a Path>,
    rules: Vec<usize>,
}

// A simplint rule, the Ruff rules on the other side, and code on which those report whenever they apply to a file, so
// a Ruff run over it tells whether Ruff's settings turn them on for that file. `undoes_fixes` tells a Ruff fix that
// reverts simplint's from a Ruff report that only asks for what simplint removes. `fix` names the recommended way out
// first.
struct Clash {
    simplint: Rule,
    ruff: &'static [RuffRule],
    probe: &'static str,
    acts_on: fn(&FileSettings, &LinterSettings) -> bool,
    undoes_fixes: bool,
    why: fn(&Names) -> String,
    fix: fn(&Names) -> String,
}

// The rules as the message shows them, and as the selectors that turn them off.
struct Names {
    by_name: bool,
    simplint: String,
    simplint_selector: String,
    ruff: String,
    ruff_selectors: String,
    many: bool,
}

impl Names {
    // The verb for the Ruff rules: `one` when a single rule works against simplint's, `many` when several do.
    fn verb(&self, one: &'static str, many: &'static str) -> &'static str {
        if self.many { many } else { one }
    }
}

const CLASHES: [Clash; 7] = [
    Clash {
        simplint: Rule::FunctionDocstring,
        ruff: &[
            RuffRule::UndocumentedPublicClass,
            RuffRule::UndocumentedPublicMethod,
            RuffRule::UndocumentedPublicFunction,
            RuffRule::UndocumentedMagicMethod,
            RuffRule::UndocumentedPublicNestedClass,
            RuffRule::UndocumentedPublicInit,
        ],
        probe: "class Class:\n    class Nested:\n        pass\n\n    \
                def __init__(self):\n        pass\n\n    def __eq__(\
                self, other):\n        pass\n\n    def method(self):\n        pass\n\n\ndef function():\n    pass\n",
        acts_on: |_, _| true,
        undoes_fixes: false,
        why: |names| {
            format!(
                "{} removes the docstrings of internal functions, classes and methods; {} {} for them",
                names.simplint_selector,
                names.ruff_selectors,
                names.verb("asks", "ask"),
            )
        },
        fix: |names| {
            format!(
                "ignore {} in Ruff's `lint.ignore` and let {} ask for documentation of the public API, or ignore {} in \
                 `[tool.simplint]`",
                names.ruff_selectors,
                shown(names.by_name, Rule::UndocumentedPublicApi.name(), Rule::UndocumentedPublicApi.code()),
                names.simplint_selector,
            )
        },
    },
    Clash {
        simplint: Rule::TestModuleDocstring,
        ruff: &[RuffRule::UndocumentedPublicModule, RuffRule::UndocumentedPublicPackage],
        probe: "x = 1\n",
        acts_on: |settings, _| settings.is_test,
        undoes_fixes: false,
        why: |names| {
            format!(
                "{} removes the docstrings of test modules; {} {} for them",
                names.simplint_selector,
                names.ruff_selectors,
                names.verb("asks", "ask"),
            )
        },
        fix: |names| {
            format!(
                "ignore {} for test files in Ruff's `lint.per-file-ignores`, or ignore {} in `[tool.simplint]`",
                names.ruff_selectors, names.simplint_selector,
            )
        },
    },
    Clash {
        simplint: Rule::FutureAnnotations,
        ruff: &[RuffRule::MissingRequiredImport],
        probe: "x = 1\n",
        acts_on: |settings, ruff| {
            settings.from_ruff.target_version.defers_annotations()
                && ruff.isort.required_imports.iter().any(|import| match import {
                    NameImport::ImportFrom(member) => {
                        member.module.as_deref() == Some("__future__") && member.name.name == "annotations"
                    }
                    NameImport::Import(_) => false,
                })
        },
        undoes_fixes: true,
        why: |names| {
            format!(
                "{} removes `from __future__ import annotations` on Python 3.14+; {} adds it back because \
                 `lint.isort.required-imports` lists it",
                names.simplint_selector, names.ruff_selectors,
            )
        },
        fix: |names| {
            format!(
                "drop the import from `required-imports`, or ignore {} in `[tool.simplint]`",
                names.simplint_selector
            )
        },
    },
    Clash {
        simplint: Rule::Utf8EncodingDefault,
        ruff: &[RuffRule::UnspecifiedEncoding],
        probe: "open(\"file\")\n",
        acts_on: |settings, _| settings.from_ruff.target_version >= PythonVersion::PY315,
        undoes_fixes: true,
        why: |names| {
            format!(
                "{} removes `encoding=\"utf-8\"` on Python 3.15+, where UTF-8 is the default; {} adds it back",
                names.simplint_selector, names.ruff_selectors,
            )
        },
        fix: ignore_in_ruff_or_simplint,
    },
    Clash {
        simplint: Rule::AbsoluteSiblingImport,
        ruff: &[RuffRule::RelativeImports],
        probe: "from . import sibling\n",
        acts_on: |_, _| true,
        undoes_fixes: true,
        why: |names| {
            format!(
                "{} makes imports from the file's own package relative; {} turns them back into absolute ones because \
                 `lint.flake8-tidy-imports.ban-relative-imports` is `\"all\"`",
                names.simplint_selector, names.ruff_selectors,
            )
        },
        fix: |names| {
            format!(
                "set `ban-relative-imports` to `\"parents\"`, or ignore {} in `[tool.simplint]`",
                names.simplint_selector
            )
        },
    },
    Clash {
        simplint: Rule::RstReturns,
        ruff: &[RuffRule::DocstringMissingReturns, RuffRule::DocstringMissingYields],
        probe: "def function(value):\n    \
                \"\"\"Return the value.\n\n    :returns: The value.\n    \"\"\"\n    return value\n\n\n\
                def generator():\n    \"\"\"Count.\n\n    :yields: The numbers.\n    \"\"\"\n    yield 1\n",
        acts_on: |settings, _| settings.project.docstring_style == Some(DocstringStyle::Rst),
        undoes_fixes: false,
        why: |names| {
            format!(
                "{} asks for the `:returns:` and `:yields:` fields of reStructuredText docstrings; {} {} them as \
                 missing sections",
                names.simplint_selector,
                names.ruff_selectors,
                names.verb("reads", "read"),
            )
        },
        fix: ignore_in_ruff_or_simplint,
    },
    Clash {
        simplint: Rule::RstMissingRaises,
        ruff: &[RuffRule::DocstringMissingException],
        probe: "def function(value):\n    \"\"\"Check the value.\n\n    :raises ValueError: When it is empty.\n    \
                \"\"\"\n    if not value:\n        raise ValueError\n",
        acts_on: |settings, _| settings.project.docstring_style == Some(DocstringStyle::Rst),
        undoes_fixes: false,
        why: |names| {
            format!(
                "{} asks for the `:raises:` fields of reStructuredText docstrings; {} reads them as a missing section",
                names.simplint_selector, names.ruff_selectors,
            )
        },
        fix: ignore_in_ruff_or_simplint,
    },
];

// How a rule is shown in the message: by its name with its code, or by its code with its name.
fn shown(by_name: bool, name: &str, code: &str) -> String {
    if by_name { format!("{name} ({code})") } else { format!("{code} ({name})") }
}

// The way out of a clash that a Ruff rule's ignore and simplint's own ignore both offer.
fn ignore_in_ruff_or_simplint(names: &Names) -> String {
    format!(
        "ignore {} in Ruff's `lint.ignore`, or ignore {} in `[tool.simplint]`",
        names.ruff_selectors, names.simplint_selector,
    )
}
