use test_case::test_case;

use crate::common;

#[test_case("single-letter-name", "single_letter_name.py" ; "single_letter_name")]
#[test_case("module-variable-case", "module_variable_case.py" ; "module_variable_case")]
#[test_case("module-variable-case", "module_variable_case_noall.py" ; "module_variable_case_without_all")]
#[test_case("module-variable-case", "module_variable_case_scopes.py" ; "module_variable_case_shadowed")]
#[test_case("module-variable-case", "module_variable_case_rename.py" ; "module_variable_case_rename_across_scopes")]
#[test_case("module-variable-case", "module_variable_case_call.py" ; "module_variable_case_dynamic_all")]
#[test_case("module-variable-case", "module_variable_case_ann.py" ; "module_variable_case_annotated_all")]
#[test_case("module-variable-case", "module_variable_case_conflict.py" ; "module_variable_case_conflict")]
#[test_case("module-variable-case", "module_variable_case_kinds.py" ; "module_variable_case_binding_kinds")]
#[test_case("module-variable-case", "module_variable_case_strings.py" ; "module_variable_case_named_in_strings")]
#[test_case("unexported-public-name", "unexported_public_name.py" ; "unexported_public_name")]
#[test_case("unexported-public-name", "unexported_public_name_extend.py" ; "unexported_public_name_extended_all")]
#[test_case("unexported-public-name", "unexported_public_name_remove.py" ; "unexported_public_name_dynamic_all")]
#[test_case("unexported-public-name", "tests/test_module_docstring.py" ; "unexported_public_name_test_module")]
#[test_case("unexported-public-name", "multiline_module_docstring_ok.py" ; "unexported_public_name_without_all")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

#[test_case("docs/conf.py", "All checks passed!\n" ; "sphinx_conf")]
#[test_case(
    "docs/settings.py",
    "docs/settings.py:1:1: SMP102 Module-scope variable `project` should be named `_PROJECT`\ndocs/settings.py:2:1: \
     SMP102 Module-scope variable `extensions` should be named `_EXTENSIONS`\nFound 2 errors.\n2 unsafe fixes \
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
