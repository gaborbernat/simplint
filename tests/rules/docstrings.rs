use test_case::test_case;

use crate::common;

#[test_case("function-docstring", "_internal/function_docstring.py" ; "function_docstring")]
#[test_case("function-docstring", "function_docstring_exported.py" ; "function_docstring_exported")]
#[test_case("function-docstring", "function_docstring_dynamic.py" ; "function_docstring_dynamic_all")]
#[test_case("function-docstring", "function_docstring_contract.py" ; "function_docstring_contract")]
#[test_case("function-docstring", "function_docstring_fields.py" ; "function_docstring_fields")]
#[test_case("function-docstring", "function_docstring_inherited.py" ; "function_docstring_inherited")]
#[test_case("function-docstring", "function_docstring_runtime.py" ; "function_docstring_runtime")]
#[test_case("function-docstring", "undocumented_public_api.py" ; "function_docstring_public_api")]
#[test_case("function-docstring", "undocumented_public_api_imported.py" ; "function_docstring_imported")]
#[test_case("function-docstring", "switched.py" ; "function_docstring_top_level_blocks")]
#[test_case("function-docstring", "facade/_core.py" ; "function_docstring_reexported_through_all")]
#[test_case("function-docstring", "facade/_helpers.py" ; "function_docstring_shared_internally")]
#[test_case("test-module-docstring", "tests/test_module_docstring.py" ; "test_module_docstring")]
#[test_case("test-module-docstring", "tests/test_module_docstring_inline.py" ; "test_module_docstring_inline")]
#[test_case("test-module-docstring", "multiline_module_docstring.py" ; "test_module_docstring_non_test")]
#[test_case("undocumented-public-api", "undocumented_public_api.py" ; "undocumented_public_api")]
#[test_case("undocumented-public-api", "undocumented_public_api_imported.py" ; "undocumented_public_api_imported")]
#[test_case("undocumented-public-api", "undocumented_public_api_inherited.py" ; "undocumented_public_api_inherited")]
#[test_case("undocumented-public-api", "undocumented_public_api_type_alias.py" ; "undocumented_public_api_type_alias")]
#[test_case("undocumented-public-api", "undocumented_public_api_ruff_overlap.py" ; "undocumented_public_api_ruff_overlap")]
#[test_case("undocumented-public-api", "undocumented_public_api_all_forms.py" ; "undocumented_public_api_all_forms")]
#[test_case("undocumented-public-api", "function_docstring_dynamic.py" ; "undocumented_public_api_dynamic_all")]
#[test_case("undocumented-public-api", "tests/test_module_docstring.py" ; "undocumented_public_api_test_module")]
#[test_case("undocumented-public-api", "facade/_core.py" ; "undocumented_public_api_reexported_through_all")]
#[test_case("undocumented-public-api", "facade/_helpers.py" ; "undocumented_public_api_shared_internally")]
#[test_case("undocumented-public-api", "loose/_impl.py" ; "undocumented_public_api_redundant_alias")]
#[test_case("undocumented-public-api", "loose/_stars.py" ; "undocumented_public_api_star_import")]
#[test_case("undocumented-public-api", "loose/_extra.py" ; "undocumented_public_api_star_import_without_all")]
#[test_case("undocumented-public-api", "loose/_deep.py" ; "undocumented_public_api_chain")]
#[test_case("undocumented-public-api", "switched.py" ; "undocumented_public_api_top_level_blocks")]
#[test_case("long-docstring-paragraph", "long_docstring_paragraph.py" ; "long_docstring_paragraph")]
#[test_case("rst-field-syntax", "rst_field_syntax.py" ; "rst_field_syntax")]
#[test_case("rst-inline-markup", "rst_inline_markup.py" ; "rst_inline_markup")]
#[test_case("rst-inline-markup", "rst_inline_markup_first_line.py" ; "rst_inline_markup_first_line")]
#[test_case("rst-type-field", "rst_type_field.py" ; "rst_type_field")]
#[test_case("rst-restating-field", "rst_restating_field.py" ; "rst_restating_field")]
#[test_case("rst-empty-field", "rst_empty_field.py" ; "rst_empty_field")]
#[test_case("rst-private-field", "rst_private_field.py" ; "rst_private_field")]
#[test_case("rst-missing-param", "rst_missing_param.py" ; "rst_missing_param")]
#[test_case("rst-missing-param", "rst_missing_param_quoted_classvar.py" ; "rst_missing_param_quoted_classvar")]
#[test_case("rst-unknown-param", "rst_unknown_param.py" ; "rst_unknown_param")]
#[test_case("rst-unknown-param", "rst_unknown_param_quotes.py" ; "rst_unknown_param_quotes")]
#[test_case("rst-duplicate-param", "rst_duplicate_param.py" ; "rst_duplicate_param")]
#[test_case("rst-param-order", "rst_param_order.py" ; "rst_param_order")]
#[test_case("rst-missing-attribute", "rst_missing_attribute.py" ; "rst_missing_attribute")]
#[test_case("rst-missing-attribute", "rst_missing_attribute_nested_class.py" ; "rst_missing_attribute_nested_class")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute.py" ; "rst_unknown_attribute")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute_delete.py" ; "rst_unknown_attribute_delete")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute_type_alias.py" ; "rst_unknown_attribute_type_alias")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute_slots.py" ; "rst_unknown_attribute_slots")]
#[test_case("rst-attribute-kind", "rst_attribute_kind.py" ; "rst_attribute_kind")]
#[test_case("rst-attribute-kind", "rst_attribute_kind_final.py" ; "rst_attribute_kind_final")]
#[test_case("rst-attribute-kind", "rst_attribute_kind_quoted_classvar.py" ; "rst_attribute_kind_quoted_classvar")]
#[test_case("rst-returns", "rst_returns.py" ; "rst_returns")]
#[test_case("rst-returns", "rst_returns_lambda_yield.py" ; "rst_returns_lambda_yield")]
#[test_case("rst-missing-raises", "rst_missing_raises.py" ; "rst_missing_raises")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

// `ruff format` keeps blank lines after a module of comments only, and Ruff reports blank lines ending the file.
#[test]
fn test_module_docstring_removal_leaves_no_blank_lines_ending_the_file() {
    let root = common::project(&[
        ("pyproject.toml", ""),
        ("tests/__init__.py", "# Licensed under the license.\n#\n\n\n\"\"\"Tests.\n\nMore.\n\"\"\"\n"),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP902"]);
    assert_eq!(common::read(root.path(), "tests/__init__.py"), "# Licensed under the license.\n#\n");
}

// A comment after a docstring that other code follows stays on a line of its own, unless it is a pragma, which goes
// with the docstring's line.
#[test_case(
    "module.py",
    "def _acquire() -> int:\n    \"\"\"Take the lock.\"\"\"  # callers must already hold the registry mutex\n    return 1\n",
    "SMP901",
    "def _acquire() -> int:\n    # callers must already hold the registry mutex\n    return 1\n" ;
    "function"
)]
#[test_case(
    "tests/test_net.py",
    "\"\"\"Network tests.\"\"\"  # these need the staging VPN\nimport os\n",
    "SMP902",
    "# these need the staging VPN\nimport os\n" ;
    "test_module"
)]
#[test_case(
    "module.py",
    "def _run() -> None:\n    \"\"\"Run the job.\"\"\"  # noqa: E501\n    print()\n",
    "SMP901",
    "def _run() -> None:\n    print()\n" ;
    "pragma"
)]
fn docstring_comment_stays_after_removal(file: &str, source: &str, select: &str, fixed: &str) {
    let root = common::project(&[("pyproject.toml", ""), (file, source)]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", select]);
    assert_eq!(common::read(root.path(), file), fixed);
}

// A class body that reads `__doc__` reads its own docstring, which a method does not: the method's `__doc__` is the
// module's.
#[test_case(
    "class Sync:\n    \"\"\"Synchronise the mirror.\"\"\"\n\n    help = __doc__\n",
    "All checks passed!\n" ;
    "class_body_reads_it"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronise the mirror.\"\"\"\n\n    def help(self) -> str:\n        return __doc__\n",
    "module.py:2:5: SMP901 Class docstring on internal code; let the name, signature and types document it\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "method_reads_module_doc"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronise the mirror.\"\"\"\n\n    fetch = lambda: 1\n",
    "module.py:2:5: SMP901 Class docstring on internal code; let the name, signature and types document it\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "lambda_reads_nothing"
)]
fn class_body_reading_doc_keeps_its_docstring(source: &str, stdout: &str) {
    let root = common::project(&[("pyproject.toml", ""), ("module.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP901"]).1, stdout);
}

// The comment wraps to the shorter of Ruff's line and doc line limits at the indentation `ruff format` gives the body,
// so a body indented by two spaces wraps the same.
#[test_case("    " ; "formatted")]
#[test_case("  " ; "two_spaces")]
fn function_docstring_comment_wraps_to_ruff_limits(indent: &str) {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[tool.ruff]\nline-length = 40\n[tool.ruff.lint]\nselect = [\"E501\", \"W505\"]\n\
             [tool.ruff.lint.pycodestyle]\nmax-doc-length = 30\n",
        ),
        (
            "module.py",
            &format!(
                "def _run() -> None:\n{indent}\"\"\"Retry once, as the first call after a restart times out.\"\"\"\n{indent}print()\n"
            ),
        ),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP901"]);
    assert_eq!(
        common::read(root.path(), "module.py"),
        format!(
            "def _run() -> None:\n{indent}# Retry once, as the first\n{indent}# call after a restart\n{indent}# times out.\n{indent}print()\n"
        )
    );
}

// Code in a docstring reads as commented-out code once it is a comment, which Ruff's ERA001 reports.
#[test]
fn function_docstring_comment_is_withdrawn_when_ruff_reports_it() {
    let source =
        "def _run() -> None:\n    \"\"\"Call it as shown below.\n\n    result = run(config)\n    \"\"\"\n    print()\n";
    let root =
        common::project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"ERA001\"]\n"), ("module.py", source)]);
    let stdout = common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP901"]).1;
    assert_eq!(
        (stdout, common::read(root.path(), "module.py")),
        (
            "module.py:2:5: SMP901 Function docstring on internal code; let the name, signature and types document it\n\
             Found 1 error.\n"
                .to_owned(),
            source.to_owned()
        )
    );
}

#[test]
fn function_docstring_comment_keeps_the_line_endings() {
    let source = "def _run() -> None: \"\"\"Retry once, as the first call\r\n    after a restart times out.\"\"\"\r\n";
    let root = common::project(&[("pyproject.toml", ""), ("module.py", source)]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP901"]);
    assert_eq!(
        common::read(root.path(), "module.py"),
        "def _run() -> None:\r\n    # Retry once, as the first call\r\n    # after a restart times out.\r\n    ...\r\n"
    );
}

// Fixes in one pass that touch wait for the next, so removing the statement after the docstring does not leave a body
// of comments only.
#[test]
fn function_docstring_comment_waits_for_a_removed_statement() {
    let root = common::project(&[
        ("pyproject.toml", ""),
        (
            "test_module.py",
            "def test_probe() -> None:\n    \"\"\"Check that adding one more test keeps the run stable.\"\"\"\n    assert True\n",
        ),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP901,SMP804"]);
    assert_eq!(
        common::read(root.path(), "test_module.py"),
        "def test_probe() -> None:\n    # Check that adding one more test keeps the run stable.\n    pass\n"
    );
}

// attrs names a field's constructor parameter by its `alias`, or by its name without the leading underscores, where a
// dataclass keeps the name.
#[test_case(
    "from attrs import define\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param x: Where it sits.\n    \"\"\"\n\n    _x: int\n",
    "All checks passed!\n" ;
    "attrs_without_underscores"
)]
#[test_case(
    "from attrs import define, field\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param pos: Where it sits.\n    \"\"\"\n\n    _x: int = field(alias=\"pos\")\n",
    "All checks passed!\n" ;
    "attrs_alias"
)]
#[test_case(
    "from dataclasses import dataclass\n\n@dataclass\nclass Point:\n    \"\"\"A point.\n\n    :param _x: Where it sits.\n    \"\"\"\n\n    _x: int\n",
    "All checks passed!\n" ;
    "dataclass_keeps_name"
)]
#[test_case(
    "from attrs import define, field\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param x: Where it sits.\n    \"\"\"\n\n    _x: int = field(alias=\"pos\")\n",
    "model.py:7:6: SMP917 `:param x:` documents `x`, which `Point` does not take\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "attrs_alias_replaces_name"
)]
fn constructor_parameters_follow_the_generator(source: &str, stdout: &str) {
    let root =
        common::project(&[("pyproject.toml", "[tool.simplint]\ndocstring-style = \"rst\"\n"), ("model.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP916,SMP917"]).1, stdout);
}

// attrs gives the `__init__` parameter of a private field the name without its underscore, so a public class documents
// that name.
#[test]
fn attrs_private_field_is_a_documented_parameter() {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\n\n[tool.simplint]\nproject-kind = \"library\"\ndocstring-style = \"rst\"\n",
        ),
        (
            "model.py",
            "from attrs import define\n\n__all__ = [\"Point\"]\n\n\n@define\nclass Point:\n    \"\"\"A point on the horizontal axis.\"\"\"\n\n    _x: int\n",
        ),
    ]);
    assert_eq!(
        common::check(root.path(), &["--select", "SMP916"]).1,
        "model.py:8:5: SMP916 Docstring of `Point` does not document parameter `x`; add `:param x:`\nFound 1 error.\n"
    );
}

// A placeholder body, as in an abstract method, says nothing of what it returns or yields, so its docstring is not asked
// for the fields of a value it has not written. A body with code is held to the same rules.
#[test_case(
    "from collections.abc import Iterator\n\n\ndef rows() -> Iterator[str]:\n    \"\"\"Read the rows.\n\n    :yields: Each row.\n    \"\"\"\n    ...\n",
    "All checks passed!\n" ;
    "placeholder_yields"
)]
#[test_case(
    "def rows():\n    \"\"\"Read the rows.\n\n    :yields: Each row.\n    \"\"\"\n    raise NotImplementedError\n",
    "All checks passed!\n" ;
    "placeholder_raises"
)]
#[test_case(
    "def total() -> int:\n    \"\"\"Add the values.\n    \"\"\"\n    return 1\n",
    "model.py:2:5: SMP923 `total` returns a value, yet its docstring has no `:returns:` field\nFound 1 error.\n" ;
    "computed_value"
)]
#[test_case(
    "def rows() -> list[str]:\n    \"\"\"Read the rows.\n\n    :yields: Each row.\n    \"\"\"\n    return []\n",
    "model.py:2:5: SMP923 `rows` returns a value, yet its docstring has no `:returns:` field\nmodel.py:4:6: SMP923 `rows` is not a generator, yet its docstring has a `:yields:` field\nFound 2 errors.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n" ;
    "computed_yields"
)]
fn placeholder_body_documents_no_return(source: &str, stdout: &str) {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\n\n[tool.simplint]\nproject-kind = \"library\"\ndocstring-style = \"rst\"\n",
        ),
        ("model.py", source),
    ]);
    assert_eq!(common::check(root.path(), &["--select", "SMP923"]).1, stdout);
}

// Without `docstring-style`, the field lists of a project's docstrings are not known to be reStructuredText.
#[test]
fn rst_rules_need_the_docstring_style() {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[build-system]\nrequires = []\n"),
        (
            "demo.py",
            "def run(path: str) -> str:\n    \"\"\"Run.\n\n    :parm path: Where.\n    \"\"\"\n    return path\n",
        ),
    ]);
    assert_eq!(common::check(root.path(), &["--select", "SMP91,SMP92"]).1, "All checks passed!\n");
}

// Renaming an alias changes the docstring, which code may read through `__doc__`, so it is an unsafe fix like the rule's
// others.
#[test]
fn alias_rename_is_an_unsafe_fix() {
    let source = "def run(path: str) -> None:\n    \"\"\"Run.\n\n    :arg path: Where.\n    \"\"\"\n    print(path)\n";
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\n[build-system]\nrequires = []\n[tool.simplint]\ndocstring-style = \"rst\"\n",
        ),
        ("demo.py", source),
    ]);
    let stdout = common::check(root.path(), &["--fix", "--select", "SMP910"]).1;
    assert_eq!(
        (stdout, common::read(root.path(), "demo.py")),
        (
            "demo.py:4:6: SMP910 Field `:arg path:` uses the alias `arg`; write `param`\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n".to_owned(),
            source.to_owned()
        )
    );
}

// Renaming an alias edits the field another rule's fix rewrites or removes, so one of the two waits for the next pass.
#[test_case(
    "def run(path: str) -> None:\n    \"\"\"Run.\n\n    :param path: Where.\n    :arg gone: Removed.\n    \"\"\"\n    print(path)\n",
    "def run(path: str) -> None:\n    \"\"\"Run.\n\n    :param path: Where.\n    \"\"\"\n    print(path)\n" ;
    "removed_field"
)]
#[test_case(
    "def run(count: int):\n    \"\"\"Run.\n\n    :param count: How far.\n    :return: Each number.\n    \"\"\"\n    yield from range(count)\n",
    "def run(count: int):\n    \"\"\"Run.\n\n    :param count: How far.\n    :yields: Each number.\n    \"\"\"\n    yield from range(count)\n" ;
    "renamed_field"
)]
fn alias_fix_composes(source: &str, fixed: &str) {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\n[build-system]\nrequires = []\n[tool.simplint]\ndocstring-style = \"rst\"\n",
        ),
        ("demo.py", source),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP910,SMP917,SMP923"]);
    assert_eq!(common::read(root.path(), "demo.py"), fixed);
}

// Ruff's D101, D102, D103 and D106 report the undocumented public definitions of their kind, which SMP903 leaves to them
// while the project's Ruff configuration runs that code.
#[test_case("def f() -> None:\n    pass\n", "D103", "Public function `f`" ; "function")]
#[test_case("class C:\n    pass\n", "D101", "Public class `C`" ; "class")]
#[test_case("class C:\n    class N:\n        pass\n", "D106", "Public class `C.N`" ; "nested_class")]
#[test_case("class C:\n    def m(self) -> None:\n        pass\n", "D102", "Public method `C.m`" ; "method")]
fn undocumented_public_api_follows_ruff_pydocstyle(source: &str, code: &str, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            (
                "pyproject.toml",
                &format!("[tool.simplint]\nproject-kind = \"library\"\n[tool.ruff.lint]\nselect = [{select}]\n"),
            ),
            ("module.py", source),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP903"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
}

// A public class of a library inherits the members of its private base without overriding them, so users reach them
// through the public class: their docstrings are documentation, and an undocumented one is reported. The override is
// the public class's own, so the base's docstring of it stays internal.
#[test]
fn inherited_members_of_a_private_base_are_public_api() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (
            "pkg/_xdg.py",
            "class XDGMixin:\n    @property\n    def user_data_dir(self) -> str:\n        \"\"\"Data directory.\"\"\"\n        \
             return \"/data\"\n\n    def site_dir(self) -> str:\n        return \"/site\"\n\n    def ensure(self, path: \
             str) -> str:\n        \"\"\"Make it.\"\"\"\n        return path\n",
        ),
        (
            "pkg/unix.py",
            "from ._xdg import XDGMixin\n\n__all__ = [\"Unix\"]\n\n\nclass Unix(XDGMixin):\n    \"\"\"Unix.\"\"\"\n\n    \
             def ensure(self, path: str) -> str:\n        \"\"\"Unix override.\"\"\"\n        return path\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP901,SMP903"]);
    assert_eq!(
        stdout,
        common::native(
            "pkg/_xdg.py:7:9: SMP903 Public method `XDGMixin.site_dir` has no docstring; document it for its users\n\
             pkg/_xdg.py:11:9: SMP901 Function docstring on internal code; let the name, signature and types document it\n\
             Found 2 errors.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}
