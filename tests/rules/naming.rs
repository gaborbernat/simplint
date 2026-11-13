use test_case::test_case;

use crate::common;

#[test_case(
    "pkg/main.py", "[project.scripts]\ncli = \"pkg.main:run\"\n", "def run() -> None:\n    pass\n", 1 ; "console_script"
)]
#[test_case("main.py", "[project.scripts]\ncli = \"main:run\"\n", "def run() -> None:\n    pass\n", 1 ; "flat_script")]
#[test_case(
    "src/app/main.py",
    "[project.scripts]\ncli = \"app.main:run\"\n",
    "def run() -> None:\n    pass\n",
    1 ;
    "namespace_script"
)]
#[test_case(
    "pkg/_main.py",
    "[project.gui-scripts]\ncli = \"pkg._main:run\"\n",
    "def run() -> None:\n    pass\n",
    1 ;
    "gui_script"
)]
#[test_case(
    "pkg/main.py",
    "[project.entry-points.example]\nplugin = \"pkg.main:run\"\n",
    "def run() -> None:\n    pass\n",
    0 ;
    "plugin_not_main"
)]
#[test_case("pkg/main.py", "", "def run() -> None:\n    pass\n", 0 ; "ordinary_module")]
#[test_case("pkg/main.py", "", "if __name__ == \"__main__\":\n    run()\n", 1 ; "direct_script")]
#[test_case("pkg/_main.py", "", "if \"__main__\" == __name__:\n    run()\n", 1 ; "reversed_guard")]
#[test_case(
    "pkg/__main__.py",
    "[project.scripts]\ncli = \"pkg.__main__:run\"\n",
    "def run() -> None:\n    pass\n",
    0 ;
    "preferred_name"
)]
#[test_case(
    "pkg/_main.py",
    "[project.scripts]\ncli = \"pkg.__main__:run\"\n",
    "def run() -> None:\n    pass\n",
    0 ;
    "implementation_module"
)]
fn entrypoint_module_name(file: &str, metadata: &str, source: &str, expected: usize) {
    let pyproject = format!("[project]\nname = \"pkg\"\nversion = \"1\"\n{metadata}");
    let root = common::project(&[("pyproject.toml", &pyproject), ("pkg/__init__.py", ""), (file, source)]);
    common::assert_report_count(root.path(), "SMP105", expected);
}

#[test_case("", "main()", 0 ; "name_alone_main")]
#[test_case("", "run()", 0 ; "name_alone_run")]
#[test_case("", "populate_cache()", 0 ; "ordinary_cache")]
#[test_case("#!/usr/bin/env python\n", "launch()", 1 ; "shebang")]
#[test_case("#!/usr/bin/env python\n", "sys.exit(launch())", 1 ; "exit_wrapper")]
#[test_case("#!/usr/bin/env python\n", "raise SystemExit(launch())", 1 ; "raise_wrapper")]
#[test_case("#!/usr/bin/env python\n", "sys.exit(0)", 1 ; "exit_without_call")]
#[test_case("#!/usr/bin/env python\n", "if __name__ == \"__main__\":\n    launch()", 0 ; "guarded")]
#[test_case("#!/usr/bin/env python\n", "launch()\nvalue = 1", 0 ; "not_last")]
#[test_case("#!/usr/bin/env python\n", "cache = populate_cache()", 0 ; "cache_assignment")]
#[test_case("", "register(lambda: main())", 0 ; "deferred_in_ordinary_module")]
#[test_case("#!/usr/bin/env python\n", "raise ValueError()", 0 ; "ordinary_raise")]
fn trailing_entrypoint_call_needs_guard(prefix: &str, trailing: &str, expected: usize) {
    let root = common::project(&[(
        "command.py",
        &format!(
            "{prefix}def main() -> None:\n    pass\n\n\ndef run() -> None:\n    pass\n\n\ndef launch() -> None:\n    \
                pass\n\n\n{trailing}\n"
        ),
    )]);
    common::assert_report_count(root.path(), "SMP106", expected);
}

#[cfg(unix)]
#[test]
fn non_utf8_script_path_has_no_target() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = common::project(&[(
        "pyproject.toml",
        "[project]\nname = \"pkg\"\nversion = \"1\"\n[project.scripts]\ncli = \"pkg.cli:run\"\n",
    )]);
    std::fs::write(root.path().join(OsString::from_vec(b"cli-\xff.py".to_vec())), "run()\n").unwrap();
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP106"]);
    assert!(!stdout.contains("SMP106"), "{stdout}");
}

#[test_case("main()\n", "if __name__ == \"__main__\":\n    main()\n" ; "main")]
#[test_case("run()\n", "if __name__ == \"__main__\":\n    run()\n" ; "run")]
#[test_case("sys.exit(run())\n", "if __name__ == \"__main__\":\n    sys.exit(run())\n" ; "exit_wrapper")]
#[test_case(
    "raise SystemExit(main())\n", "if __name__ == \"__main__\":\n    raise SystemExit(main())\n" ; "raise_wrapper"
)]
#[test_case("run()  # launch\n", "if __name__ == \"__main__\":\n    run()  # launch\n" ; "inline_comment")]
#[test_case("run()\r\n", "if __name__ == \"__main__\":\r\n    run()\r\n" ; "crlf")]
#[test_case("run()", "if __name__ == \"__main__\":\n    run()" ; "no_final_newline")]
fn trailing_entrypoint_call_autofix(source: &str, expected: &str) {
    let shebang = if source.contains("\r\n") { "#!/usr/bin/env python\r\n" } else { "#!/usr/bin/env python\n" };
    let root = common::project(&[("command.py", &format!("{shebang}{source}"))]);
    let (code, stdout, _) = common::fix_unsafe(root.path(), "SMP106");
    assert_eq!((code, common::read(root.path(), "command.py")), (0, format!("{shebang}{expected}")), "{stdout}");
}

#[test_case("run(\n)\n" ; "multiline")]
#[test_case("value = 1; run()\n" ; "shared_line")]
fn trailing_entrypoint_call_without_fix(source: &str) {
    let source = format!("#!/usr/bin/env python\n{source}");
    let root = common::project(&[("command.py", &source)]);
    let (_, stdout, _) = common::fix_unsafe(root.path(), "SMP106");
    assert_eq!((stdout.contains("SMP106"), common::read(root.path(), "command.py")), (true, source), "{stdout}");
}

#[test]
fn trailing_entrypoint_call_requires_unsafe_fixes() {
    let root = common::project(&[("command.py", "#!/usr/bin/env python\nrun()\n")]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP106", "--fix"]);
    assert_eq!(
        (stdout.contains("SMP106"), common::read(root.path(), "command.py")),
        (true, "#!/usr/bin/env python\nrun()\n".to_owned()),
        "{stdout}"
    );
}

#[test_case("__main__.py" ; "root")]
#[test_case("pkg/__main__.py" ; "package")]
fn main_module_requires_guard(file: &str) {
    let root = common::project(&[(file, "from pkg.cli import launch\n\nlaunch()\n")]);
    common::assert_report_count(root.path(), "SMP106", 1);
    let (code, stdout, _) = common::fix_unsafe(root.path(), "SMP106");
    assert_eq!(
        (code, common::read(root.path(), file)),
        (0, "from pkg.cli import launch\n\nif __name__ == \"__main__\":\n    launch()\n".to_owned()),
        "{stdout}"
    );
}

#[test_case("[project.scripts]\ncli = \"pkg.cli:launch\"\n", "launch()", 1 ; "console_script")]
#[test_case("[project.gui-scripts]\ncli = \"pkg.cli:launch\"\n", "sys.exit(launch())", 1 ; "gui_script")]
#[test_case("[project.scripts]\ncli = \"pkg.cli:Cli.start\"\n", "Cli . start()", 1 ; "dotted_target")]
#[test_case("[project.scripts]\ncli = \"pkg.cli:launch\"\n", "populate_cache()", 0 ; "unrelated_call")]
#[test_case("[project.entry-points.plugins]\ncli = \"pkg.cli:launch\"\n", "launch()", 0 ; "plugin")]
fn configured_entrypoint_call(metadata: &str, trailing: &str, expected: usize) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[project]\nname = \"pkg\"\nversion = \"1\"\n{metadata}")),
        ("pkg/__init__.py", ""),
        ("pkg/cli.py", &format!("def launch() -> None:\n    pass\n\n\n{trailing}\n")),
    ]);
    common::assert_report_count(root.path(), "SMP106", expected);
}

#[test]
fn configured_entrypoint_call_autofix() {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"pkg\"\nversion = \"1\"\n[project.scripts]\ncli = \"pkg.cli:launch\"\n"),
        ("pkg/__init__.py", ""),
        ("pkg/cli.py", "def launch() -> None:\n    pass\n\n\nlaunch()\n"),
    ]);
    let (code, stdout, _) = common::fix_unsafe(root.path(), "SMP106");
    assert_eq!(
        (code, common::read(root.path(), "pkg/cli.py")),
        (0, "def launch() -> None:\n    pass\n\n\nif __name__ == \"__main__\":\n    launch()\n".to_owned()),
        "{stdout}"
    );
}

#[test]
fn package_init_console_script_call() {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"pkg\"\nversion = \"1\"\n[project.scripts]\ncli = \"pkg:launch\"\n"),
        ("pkg/__init__.py", "def launch() -> None:\n    pass\n\n\nlaunch()\n"),
    ]);
    common::assert_report_count(root.path(), "SMP106", 1);
}

#[cfg(unix)]
#[test]
fn executable_script_requires_guard() {
    let root = common::project(&[("command.py", "launch()\n")]);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP106"]);
    assert!(!stdout.contains("SMP106"), "{stdout}");
    common::set_mode(&root.path().join("command.py"), 0o755);
    let (_, stdout, _) = common::check(root.path(), &["--select", "SMP106"]);
    assert_eq!(stdout.matches("SMP106").count(), 1, "{stdout}");
}

#[test_case("single-letter-name", "single_letter_name.py", None ; "single_letter_name")]
#[test_case("meaningless-name", "meaningless_name.py", None ; "meaningless_name")]
#[test_case("catch-all-module", "utils/__init__.py", None ; "catch_all_module_package")]
#[test_case("catch-all-module", "utils/slugs.py", None ; "catch_all_module_package_module")]
#[test_case("catch-all-module", "helpers.py", None ; "catch_all_module_module")]
#[test_case("catch-all-module", "models.py", None ; "catch_all_module_outside_default_list")]
#[test_case("module-variable-case", "_module_variable_case.py", None ; "module_variable_case")]
#[test_case("module-variable-case", "_module_variable_case_noall.py", None ; "module_variable_case_without_all")]
#[test_case("module-variable-case", "module_variable_case_scopes.py", None ; "module_variable_case_shadowed")]
#[test_case(
    "module-variable-case", "_module_variable_case_rename.py", None ; "module_variable_case_rename_across_scopes"
)]
#[test_case("module-variable-case", "module_variable_case_debug.py", None ; "module_variable_case_debug")]
#[test_case("module-variable-case", "module_variable_case_call.py", None ; "module_variable_case_dynamic_all")]
#[test_case("module-variable-case", "_module_variable_case_ann.py", None ; "module_variable_case_annotated_all")]
#[test_case("module-variable-case", "module_variable_case_conflict.py", None ; "module_variable_case_conflict")]
#[test_case("module-variable-case", "module_variable_case_kinds.py", None ; "module_variable_case_binding_kinds")]
#[test_case("module-variable-case", "module_variable_case_strings.py", None ; "module_variable_case_named_in_strings")]
#[test_case("module-variable-case", "_module_variable_case_alembic.py", None ; "module_variable_case_alembic_revision")]
#[test_case("module-variable-case", "module_variable_case_gunicorn.py", None ; "module_variable_case_gunicorn_named")]
#[test_case(
    "module-variable-case", "_module_variable_case_frameworks.py", None ; "module_variable_case_framework_names"
)]
#[test_case(
    "module-variable-case", "_module_variable_case_lookalikes.py", None ; "module_variable_case_framework_lookalikes"
)]
#[test_case("module-variable-case", "_module_variable_case_source_text.py", None ; "module_variable_case_source_text")]
#[test_case("module-variable-case", "_module_variable_case_version.py", None ; "module_variable_case_version_file")]
#[test_case("module-variable-case", "_module_variable_case_ply.py", None ; "module_variable_case_ply_grammar")]
#[test_case("module-variable-case", "module_variable_case_ply_table.py", None ; "module_variable_case_ply_table")]
#[test_case("module-variable-case", "_module_variable_case_dbapi.py", None ; "module_variable_case_db_api_globals")]
#[test_case("module-variable-case", "_module_variable_case_typer.py", None ; "module_variable_case_typer_app")]
#[test_case("module-variable-case", "module_variable_case_type_alias.py", None ; "module_variable_case_type_alias")]
#[test_case("module-variable-case", "_module_variable_case_ruff.py", None ; "module_variable_case_ruff_overlap")]
#[test_case(
    "module-variable-case", "module_variable_case_function_alias.py", None ; "module_variable_case_function_alias"
)]
#[test_case("module-variable-case", "_module_variable_case_block_local.py", None ; "module_variable_case_block_local")]
#[test_case(
    "module-variable-case", "module_variable_case_computed_setattr.py", None ; "module_variable_case_computed_setattr"
)]
#[test_case(
    "module-variable-case", "module_variable_case_computed_fstring.py", None ; "module_variable_case_computed_fstring"
)]
#[test_case(
    "module-variable-case",
    "module_variable_case_computed_patch_object.py",
    None ;
    "module_variable_case_computed_patch_object"
)]
#[test_case(
    "module-variable-case",
    "_module_variable_case_computed_control.py",
    None ;
    "module_variable_case_computed_other_module"
)]
#[test_case(
    "module-variable-case",
    "_module_variable_case_read_in_two_blocks.py",
    None ;
    "module_variable_case_read_in_two_blocks"
)]
#[test_case(
    "module-variable-case",
    "module_variable_case_type_checking_stubs.py",
    None ;
    "module_variable_case_type_checking_stubs"
)]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

#[test_case("docs/conf.py", common::PASSED ; "sphinx_conf")]
#[test_case(
    "docs/settings.py",
    "docs/settings.py:1:1: SMP102 Module-scope variable `project` should be named `PROJECT`\ndocs/settings.py:2:1: \
     SMP102 Module-scope variable `extensions` should be named `EXTENSIONS`\nFound 2 errors.\n2 unsafe fixes \
     available with `--fix --unsafe-fixes`.\n" ;
    "other_module"
)]
fn sphinx_conf_settings_keep_their_names(path: &str, stdout: &str) {
    let root = common::project(&[(path, "project = \"demo\"\nextensions = [\"sphinx.ext.autodoc\"]\n")]);
    assert_eq!(common::check(root.path(), &["--select", "SMP102"]).1, common::native(stdout));
}

// A string blocks the rename only when it holds the name as a whole word.
#[test_case("misspelled spelled_out spelledout", true ; "inside_longer_words")]
#[test_case("misspelled (spelled)", false ; "between_punctuation")]
fn module_variable_rename_checks_whole_words_in_strings(text: &str, fixable: bool) {
    let root = common::project(&[("module.py", &format!("spelled = 1\nprint(\"{text}\")\n"))]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP102"]).1;
    assert_eq!(stdout.contains("SMP102 [*]"), fixable);
}

// A string of another file may spell the name, as a test that looks a module variable up by its spelling does. A module
// named like a standard library module is imported from outside the run, so its names may be read unseen.
#[test_case(
    &[("config.py", "limit = 3\nprint(limit)\n"), ("tests/test_config.py", "LOOKUP = \"limit\"\n")],
    "limit" ;
    "string_of_another_file"
)]
#[test_case(
    &[
        ("typing_extensions.py", "_typing_names = [\"Any\"]\n"),
        ("test_typing.py", "import typing_extensions\n\nNAMES = typing_extensions._typing_names\n"),
    ],
    "_typing_names" ;
    "standard_library_namesake"
)]
fn module_variable_rename_withholds_its_fix_for_unseen_reads(files: &[(&str, &str)], name: &str) {
    let root = common::project(files);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP102"]).1;
    assert_eq!(
        (stdout.contains(&format!("SMP102 Module-scope variable `{name}`")), stdout.contains("SMP102 [*]")),
        (true, false)
    );
}

// Ruff's N816 skips the names its pep8-naming `ignore-names` lists, so SMP102 keeps reporting them: without a fix
// here, nothing else would. Without N816 selected, SMP102 reports the names N816 would skip.
#[test_case("[tool.ruff.lint]\nselect = [\"N816\"]\n", false ; "selected_default_names")]
#[test_case(
    "[tool.ruff.lint]\nselect = [\"N816\"]\n[tool.ruff.lint.pep8-naming]\nignore-names = [\"myValue\"]\n",
    true
    ; "selected_ignore_names"
)]
#[test_case(
    "[tool.ruff.lint]\nselect = [\"N816\"]\n[tool.ruff.lint.pep8-naming]\nextend-ignore-names = [\"my*\"]\n",
    true
    ; "selected_extend_ignore_names_glob"
)]
#[test_case("[tool.ruff.lint]\nselect = []\n", false ; "unselected")]
fn module_variable_rename_keeps_names_ruff_ignores(ruff_settings: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[project]\nrequires-python = \">=3.12\"\n{ruff_settings}")),
        ("module.py", "myValue = 1\nMYVALUE = 2\nprint(myValue, MYVALUE)\n"),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP102"]).1;
    assert_eq!(stdout.contains("SMP102 Module-scope variable `myValue`"), reported, "{stdout}");
}

// Ruff's N816 reports a mixedCase name at each of its module-scope stores, so SMP102 leaves such a name to it when it
// offers no rename, whether or not the run selects N816. A rename stays here, as does a name that N816 does not report:
// an `except` target, or one N816 ignores.
#[test_case(
    "def setup():\n    global myValue\n    myValue = 1\n\n\nmyValue = 3\nMYVALUE = 4\nprint(myValue, MYVALUE, setup)\n",
    "",
    [false, false] ;
    "module_store_left_to_n816"
)]
#[test_case(
    "def setup():\n    global myValue\n    myValue = 1\n\n\nfor myValue in range(2):\n    pass\nMYVALUE = 4\nprint(\
        MYVALUE, setup)\n",
    "",
    [false, false] ;
    "loop_target_left_to_n816"
)]
#[test_case(
    "def setup():\n    global myValue\n    myValue = 1\n\n\nmatch 1:\n    case myValue:\n        pass\nMYVALUE = 4\n\
        print(MYVALUE, setup)\n",
    "",
    [false, false] ;
    "match_capture_left_to_n816"
)]
#[test_case(
    "def setup():\n    global myValue\n    myValue = 1\n\n\ntry:\n    pass\nexcept ValueError as myValue:\n    pass\n\
        MYVALUE = 4\nprint(MYVALUE, setup)\n",
    "",
    [true, true] ;
    "except_target_kept"
)]
#[test_case("myValue = 3\nprint(myValue)\n", "", [true, true] ; "rename_kept")]
#[test_case(
    "def setup():\n    global myValue\n    myValue = 1\n\n\nmyValue = 3\nMYVALUE = 4\nprint(myValue, MYVALUE, setup)\n",
    "[tool.ruff.lint.pep8-naming]\nignore-names = [\"myValue\"]\n",
    [true, true] ;
    "ignored_name_kept"
)]
fn module_variable_case_leaves_names_without_rename_to_n816(source: &str, settings: &str, expected: [bool; 2]) {
    let case = common::RuffOverlap { settings, select: "SMP102", ruff_code: "N816", files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, "Module-scope variable `myValue`"), expected);
}

// Ruff's E741, E742 and E743 report `l`, `I` and `O` on the variables, classes and functions of their kind, so SMP101
// leaves those letters to them whether or not the project selects them. Other letters stay with the rule, and so do
// those three letters on an import alias, which no Ruff rule checks.
#[test_case("l = 1\n", "Single-letter name `l`", false ; "variable")]
#[test_case("class I:\n    pass\n", "Single-letter name `I`", false ; "class")]
#[test_case("def l() -> None:\n    pass\n", "Single-letter name `l`", false ; "function")]
#[test_case("k = 1\n", "Single-letter name `k`", true ; "other_letter")]
#[test_case("import os as l\n", "Single-letter name `l`", true ; "import_alias")]
fn single_letter_name_leaves_ambiguous_names_to_ruff(source: &str, fragment: &str, reported: bool) {
    let root = common::project(&[("module.py", source)]);
    let stdout = common::check(root.path(), &["--select", "SMP101"]).1;
    assert_eq!(stdout.contains(fragment), reported, "{stdout}");
}

#[test_case("from typing import TypeVar\n\nT = TypeVar(\"T\")\n", 0, 0 ; "type_var")]
#[test_case("from typing import ParamSpec\n\nP = ParamSpec(\"P\")\n", 0, 0 ; "param_spec")]
#[test_case("from typing import TypeVarTuple\n\nTs = TypeVarTuple(\"Ts\")\n", 0, 0 ; "type_var_tuple")]
#[test_case("import typing\n\nT = typing.TypeVar(\"T\")\n", 0, 0 ; "qualified_type_var")]
#[test_case("from typing_extensions import TypeVar as TV\n\nT = TV(\"T\")\n", 0, 0 ; "typing_extensions_alias")]
#[test_case("from typing import TypeVar\n\nT = TypeVar(name=\"T\")\n", 0, 0 ; "keyword_name")]
#[test_case("class Box[T]:\n    pass\n", 0, 0 ; "pep_695_type_parameter")]
#[test_case("class Box[x]:\n    pass\n", 1, 0 ; "lowercase_pep_695_parameter")]
#[test_case("from typing import TypeVar\n\nx = TypeVar(\"x\")\n", 1, 0 ; "lowercase_type_var")]
#[test_case("T = 1\n", 1, 1 ; "ordinary_variable")]
#[test_case("from typing import TypeVar\n\nT = TypeVar(\"U\")\n", 1, 1 ; "mismatched_name")]
#[test_case("from typing import TypeVar\n\n_NAME = \"T\"\nT = TypeVar(_NAME)\n", 1, 1 ; "nonliteral_name")]
#[test_case("from typing import TypeVar\n\nT = TypeVar()\n", 1, 1 ; "missing_name")]
#[test_case("from typing import TypeVar\n\nT = TypeVar(\"T\")\nT = 1\n", 1, 1 ; "reassigned_type_var")]
#[test_case("def TypeVar(name: str) -> str:\n    return name\n\nT = TypeVar(\"T\")\n", 1, 1 ; "shadowed_constructor")]
fn type_parameter_names(source: &str, single_letter: usize, variable_prefix: usize) {
    let root = common::project(&[("module.py", source)]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP101,SMP404"]).1;
    assert_eq!(
        (stdout.matches("SMP101").count(), stdout.matches("SMP404").count()),
        (single_letter, variable_prefix),
        "{stdout}"
    );
}
