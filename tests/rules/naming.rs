use test_case::test_case;

use crate::common;

#[test_case("pkg/main.py", "[project.scripts]\ncli = \"pkg.main:run\"\n", "def run() -> None:\n    pass\n", 1 ; "console_script")]
#[test_case("main.py", "[project.scripts]\ncli = \"main:run\"\n", "def run() -> None:\n    pass\n", 1 ; "flat_script")]
#[test_case("src/app/main.py", "[project.scripts]\ncli = \"app.main:run\"\n", "def run() -> None:\n    pass\n", 1 ; "namespace_script")]
#[test_case("pkg/_main.py", "[project.gui-scripts]\ncli = \"pkg._main:run\"\n", "def run() -> None:\n    pass\n", 1 ; "gui_script")]
#[test_case("pkg/main.py", "[project.entry-points.example]\nplugin = \"pkg.main:run\"\n", "def run() -> None:\n    pass\n", 0 ; "plugin_not_main")]
#[test_case("pkg/main.py", "", "def run() -> None:\n    pass\n", 0 ; "ordinary_module")]
#[test_case("pkg/main.py", "", "if __name__ == \"__main__\":\n    run()\n", 1 ; "direct_script")]
#[test_case("pkg/_main.py", "", "if \"__main__\" == __name__:\n    run()\n", 1 ; "reversed_guard")]
#[test_case("pkg/__main__.py", "[project.scripts]\ncli = \"pkg.__main__:run\"\n", "def run() -> None:\n    pass\n", 0 ; "preferred_name")]
#[test_case("pkg/_main.py", "[project.scripts]\ncli = \"pkg.__main__:run\"\n", "def run() -> None:\n    pass\n", 0 ; "implementation_module")]
fn entrypoint_module_name(file: &str, metadata: &str, source: &str, expected: usize) {
    let pyproject = format!("[project]\nname = \"pkg\"\nversion = \"1\"\n{metadata}");
    let root = common::project(&[("pyproject.toml", &pyproject), ("pkg/__init__.py", ""), (file, source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP105"]);
    assert_eq!(stdout.matches("SMP105").count(), expected, "{stdout}");
}

#[test_case("main()", 1 ; "direct")]
#[test_case("sys.exit(main())", 1 ; "sys_exit")]
#[test_case("raise SystemExit(main())", 1 ; "system_exit")]
#[test_case("run(result=main())", 1 ; "keyword_argument")]
#[test_case("if __name__ == \"__main__\":\n    main()", 0 ; "guarded")]
#[test_case("if \"__main__\" == __name__:\n    sys.exit(main())", 0 ; "reversed_guard")]
#[test_case("runner.main()", 0 ; "attribute")]
#[test_case("register(lambda: main())", 0 ; "deferred")]
#[test_case("main()\nprint(\"done\")", 0 ; "not_last")]
fn trailing_main_call_needs_guard(trailing: &str, expected: usize) {
    let root = common::project(&[("command.py", &format!("def main() -> None:\n    pass\n\n\n{trailing}\n"))]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP106"]);
    assert_eq!(stdout.matches("SMP106").count(), expected, "{stdout}");
}

#[test]
fn package_main_can_call_main_without_a_guard() {
    let root = common::project(&[("pkg/__main__.py", "def main() -> None:\n    pass\n\n\nmain()\n")]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP106"]);
    assert_eq!(stdout, "All checks passed!\n");
}

#[test_case("single-letter-name", "single_letter_name.py" ; "single_letter_name")]
#[test_case("meaningless-name", "meaningless_name.py" ; "meaningless_name")]
#[test_case("catch-all-module", "utils/__init__.py" ; "catch_all_module_package")]
#[test_case("catch-all-module", "utils/slugs.py" ; "catch_all_module_package_module")]
#[test_case("catch-all-module", "helpers.py" ; "catch_all_module_module")]
#[test_case("catch-all-module", "models.py" ; "catch_all_module_outside_default_list")]
#[test_case("module-variable-case", "_module_variable_case.py" ; "module_variable_case")]
#[test_case("module-variable-case", "_module_variable_case_noall.py" ; "module_variable_case_without_all")]
#[test_case("module-variable-case", "module_variable_case_scopes.py" ; "module_variable_case_shadowed")]
#[test_case("module-variable-case", "_module_variable_case_rename.py" ; "module_variable_case_rename_across_scopes")]
#[test_case("module-variable-case", "module_variable_case_debug.py" ; "module_variable_case_debug")]
#[test_case("module-variable-case", "module_variable_case_call.py" ; "module_variable_case_dynamic_all")]
#[test_case("module-variable-case", "_module_variable_case_ann.py" ; "module_variable_case_annotated_all")]
#[test_case("module-variable-case", "module_variable_case_conflict.py" ; "module_variable_case_conflict")]
#[test_case("module-variable-case", "module_variable_case_kinds.py" ; "module_variable_case_binding_kinds")]
#[test_case("module-variable-case", "module_variable_case_strings.py" ; "module_variable_case_named_in_strings")]
#[test_case("module-variable-case", "_module_variable_case_alembic.py" ; "module_variable_case_alembic_revision")]
#[test_case("module-variable-case", "module_variable_case_gunicorn.py" ; "module_variable_case_gunicorn_named")]
#[test_case("module-variable-case", "_module_variable_case_frameworks.py" ; "module_variable_case_framework_names")]
#[test_case("module-variable-case", "_module_variable_case_lookalikes.py" ; "module_variable_case_framework_lookalikes")]
#[test_case("module-variable-case", "_module_variable_case_source_text.py" ; "module_variable_case_source_text")]
#[test_case("module-variable-case", "_module_variable_case_version.py" ; "module_variable_case_version_file")]
#[test_case("module-variable-case", "_module_variable_case_ply.py" ; "module_variable_case_ply_grammar")]
#[test_case("module-variable-case", "module_variable_case_ply_table.py" ; "module_variable_case_ply_table")]
#[test_case("module-variable-case", "_module_variable_case_dbapi.py" ; "module_variable_case_db_api_globals")]
#[test_case("module-variable-case", "_module_variable_case_typer.py" ; "module_variable_case_typer_app")]
#[test_case("module-variable-case", "module_variable_case_type_alias.py" ; "module_variable_case_type_alias")]
#[test_case("module-variable-case", "_module_variable_case_ruff.py" ; "module_variable_case_ruff_overlap")]
#[test_case("module-variable-case", "module_variable_case_function_alias.py" ; "module_variable_case_function_alias")]
#[test_case("module-variable-case", "_module_variable_case_block_local.py" ; "module_variable_case_block_local")]
#[test_case("module-variable-case", "module_variable_case_computed_setattr.py" ; "module_variable_case_computed_setattr")]
#[test_case("module-variable-case", "module_variable_case_computed_fstring.py" ; "module_variable_case_computed_fstring")]
#[test_case("module-variable-case", "module_variable_case_computed_patch_object.py" ; "module_variable_case_computed_patch_object")]
#[test_case("module-variable-case", "_module_variable_case_computed_control.py" ; "module_variable_case_computed_other_module")]
#[test_case("module-variable-case", "_module_variable_case_read_in_two_blocks.py" ; "module_variable_case_read_in_two_blocks")]
#[test_case("module-variable-case", "module_variable_case_type_checking_stubs.py" ; "module_variable_case_type_checking_stubs")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

#[test_case("docs/conf.py", "All checks passed!\n" ; "sphinx_conf")]
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

// A string of another file may spell the name, as a test that looks a module variable up by its spelling does.
#[test]
fn module_variable_rename_checks_strings_of_other_files() {
    let root = common::project(&[
        ("config.py", "limit = 3\nprint(limit)\n"),
        ("tests/test_config.py", "LOOKUP = \"limit\"\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP102"]).1;
    assert!(stdout.contains("SMP102 Module-scope variable `limit`"));
    assert!(!stdout.contains("SMP102 [*]"));
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
#[test_case("[tool.ruff.lint]\nselect = []\n", true ; "unselected")]
fn module_variable_rename_keeps_names_ruff_ignores(ruff_settings: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[project]\nrequires-python = \">=3.12\"\n{ruff_settings}")),
        ("module.py", "myValue = 1\nMYVALUE = 2\nprint(myValue, MYVALUE)\n"),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP102"]).1;
    assert_eq!(stdout.contains("SMP102 Module-scope variable `myValue`"), reported, "{stdout}");
}

// A module named like a standard library module is imported from outside the run, so its names may be read unseen.
#[test]
fn module_variable_rename_keeps_names_read_off_a_standard_library_namesake() {
    let root = common::project(&[
        ("typing_extensions.py", "_typing_names = [\"Any\"]\n"),
        ("test_typing.py", "import typing_extensions\n\nNAMES = typing_extensions._typing_names\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP102"]).1;
    assert!(stdout.contains("SMP102 Module-scope variable `_typing_names`"));
    assert!(!stdout.contains("SMP102 [*]"));
}

// Ruff's E741, E742 and E743 report `l`, `I` and `O` on the variables, classes and functions of their kind. SMP101
// leaves those letters to the Ruff rule of the binding's kind while it runs.
#[test_case("l = 1\n", "E741", "Single-letter name `l`" ; "variable")]
#[test_case("class I:\n    pass\n", "E742", "Single-letter name `I`" ; "class")]
#[test_case("def l() -> None:\n    pass\n", "E743", "Single-letter name `l`" ; "function")]
fn single_letter_name_follows_ruff_ambiguous_names(source: &str, code: &str, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n")),
            ("module.py", source),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP101"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
}
