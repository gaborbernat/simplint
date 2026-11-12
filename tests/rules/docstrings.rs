use test_case::test_case;

use crate::common;

#[test_case("function-docstring", "_internal/function_docstring.py", None ; "function_docstring")]
#[test_case("function-docstring", "function_docstring_exported.py", None ; "function_docstring_exported")]
#[test_case("function-docstring", "function_docstring_dynamic.py", None ; "function_docstring_dynamic_all")]
#[test_case("function-docstring", "function_docstring_contract.py", None ; "function_docstring_contract")]
#[test_case("function-docstring", "function_docstring_fields.py", None ; "function_docstring_fields")]
#[test_case("function-docstring", "function_docstring_inherited.py", None ; "function_docstring_inherited")]
#[test_case("function-docstring", "function_docstring_runtime.py", None ; "function_docstring_runtime")]
#[test_case("function-docstring", "undocumented_public_api.py", None ; "function_docstring_public_api")]
#[test_case("function-docstring", "undocumented_public_api_imported.py", None ; "function_docstring_imported")]
#[test_case("function-docstring", "switched.py", None ; "function_docstring_top_level_blocks")]
#[test_case("function-docstring", "facade/_core.py", None ; "function_docstring_reexported_through_all")]
#[test_case("function-docstring", "facade/_helpers.py", None ; "function_docstring_shared_internally")]
#[test_case("test-module-docstring", "tests/test_module_docstring.py", None ; "test_module_docstring")]
#[test_case("test-module-docstring", "tests/test_module_docstring_inline.py", None ; "test_module_docstring_inline")]
#[test_case("test-module-docstring", "multiline_module_docstring.py", None ; "test_module_docstring_non_test")]
#[test_case("undocumented-public-api", "undocumented_public_api.py", None ; "undocumented_public_api")]
#[test_case(
    "undocumented-public-api", "undocumented_public_api_imported.py", None ; "undocumented_public_api_imported"
)]
#[test_case(
    "undocumented-public-api", "undocumented_public_api_inherited.py", None ; "undocumented_public_api_inherited"
)]
#[test_case(
    "undocumented-public-api", "undocumented_public_api_type_alias.py", None ; "undocumented_public_api_type_alias"
)]
#[test_case(
    "undocumented-public-api", "undocumented_public_api_ruff_overlap.py", None ; "undocumented_public_api_ruff_overlap"
)]
#[test_case(
    "undocumented-public-api", "undocumented_public_api_all_forms.py", None ; "undocumented_public_api_all_forms"
)]
#[test_case("undocumented-public-api", "function_docstring_dynamic.py", None ; "undocumented_public_api_dynamic_all")]
#[test_case("undocumented-public-api", "tests/test_module_docstring.py", None ; "undocumented_public_api_test_module")]
#[test_case("undocumented-public-api", "facade/_core.py", None ; "undocumented_public_api_reexported_through_all")]
#[test_case("undocumented-public-api", "facade/_helpers.py", None ; "undocumented_public_api_shared_internally")]
#[test_case("undocumented-public-api", "loose/_impl.py", None ; "undocumented_public_api_redundant_alias")]
#[test_case("undocumented-public-api", "loose/_stars.py", None ; "undocumented_public_api_star_import")]
#[test_case("undocumented-public-api", "loose/_extra.py", None ; "undocumented_public_api_star_import_without_all")]
#[test_case("undocumented-public-api", "loose/_deep.py", None ; "undocumented_public_api_chain")]
#[test_case("undocumented-public-api", "switched.py", None ; "undocumented_public_api_top_level_blocks")]
#[test_case("long-docstring-paragraph", "long_docstring_paragraph.py", None ; "long_docstring_paragraph")]
#[test_case("rst-field-syntax", "rst_field_syntax.py", None ; "rst_field_syntax")]
#[test_case("rst-field-syntax", "rst_field_syntax_docutils.py", None ; "rst_field_syntax_docutils")]
#[test_case("rst-inline-markup", "rst_inline_markup.py", None ; "rst_inline_markup")]
#[test_case("rst-inline-markup", "rst_inline_markup_first_line.py", None ; "rst_inline_markup_first_line")]
#[test_case("rst-inline-markup", "rst_inline_markup_docutils.py", None ; "rst_inline_markup_docutils")]
#[test_case("rst-inline-markup", "rst_inline_markup_scanner.py", None ; "rst_inline_markup_scanner")]
#[test_case("rst-type-field", "rst_type_field.py", None ; "rst_type_field")]
#[test_case("rst-restating-field", "rst_restating_field.py", None ; "rst_restating_field")]
#[test_case("rst-empty-field", "rst_empty_field.py", None ; "rst_empty_field")]
#[test_case("rst-private-field", "rst_private_field.py", None ; "rst_private_field")]
#[test_case("rst-missing-param", "rst_missing_param.py", None ; "rst_missing_param")]
#[test_case("rst-missing-param", "rst_missing_param_quoted_classvar.py", None ; "rst_missing_param_quoted_classvar")]
#[test_case("rst-unknown-param", "rst_unknown_param.py", None ; "rst_unknown_param")]
#[test_case("rst-unknown-param", "rst_unknown_param_quotes.py", None ; "rst_unknown_param_quotes")]
#[test_case("rst-unknown-param", "rst_unknown_param_backticks.py", None ; "rst_unknown_param_backticks")]
#[test_case("rst-duplicate-param", "rst_duplicate_param.py", None ; "rst_duplicate_param")]
#[test_case("rst-param-order", "rst_param_order.py", None ; "rst_param_order")]
#[test_case("rst-missing-attribute", "rst_missing_attribute.py", None ; "rst_missing_attribute")]
#[test_case(
    "rst-missing-attribute", "rst_missing_attribute_nested_class.py", None ; "rst_missing_attribute_nested_class"
)]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute.py", None ; "rst_unknown_attribute")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute_delete.py", None ; "rst_unknown_attribute_delete")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute_type_alias.py", None ; "rst_unknown_attribute_type_alias")]
#[test_case("rst-unknown-attribute", "rst_unknown_attribute_slots.py", None ; "rst_unknown_attribute_slots")]
#[test_case("rst-attribute-kind", "rst_attribute_kind.py", None ; "rst_attribute_kind")]
#[test_case("rst-attribute-kind", "rst_attribute_kind_final.py", None ; "rst_attribute_kind_final")]
#[test_case("rst-attribute-kind", "rst_attribute_kind_quoted_classvar.py", None ; "rst_attribute_kind_quoted_classvar")]
#[test_case("rst-returns", "rst_returns.py", None ; "rst_returns")]
#[test_case("rst-returns", "rst_returns_lambda_yield.py", None ; "rst_returns_lambda_yield")]
#[test_case("rst-missing-raises", "rst_missing_raises.py", None ; "rst_missing_raises")]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
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
    "def _acquire() -> int:\n    \"\"\"Take the lock.\"\"\"  # callers must already hold the registry mutex\n    \
        return 1\n",
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
#[test_case(
    "module.py",
    "def _run() -> None:\n    \"\"\"Run the job.\"\"\"  # fmt: off\n    print()\n",
    "SMP901",
    "def _run() -> None:\n    print()\n" ;
    "fmt_directive"
)]
#[test_case(
    "module.py",
    "def _run() -> None:\n    \"\"\"Run the job.\"\"\"  # first # noqa: E501\n    print()\n",
    "SMP901",
    "def _run() -> None:\n    print()\n" ;
    "comment_ending_in_pragma"
)]
#[test_case(
    "tests/test_net.py",
    "\"\"\"Network tests.\"\"\"  # fmt: skip\nimport os\n",
    "SMP902",
    "import os\n" ;
    "test_module_directive"
)]
#[test_case(
    "module.py",
    "def _run() -> None:\n    \"\"\"Run the job.\"\"\"  # type: ignore\n",
    "SMP901",
    "def _run() -> None:\n    ...\n" ;
    "single_statement_directive"
)]
fn docstring_comment_stays_after_removal(file: &str, source: &str, select: &str, fixed: &str) {
    let root = common::project(&[("pyproject.toml", ""), (file, source)]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", select]);
    assert_eq!(common::read(root.path(), file), fixed);
}

// A class body that reads `__doc__` reads its own docstring, which a method does not: the method's `__doc__` is the
// module's.
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    help = __doc__\n",
    common::PASSED ;
    "class_body_reads_it"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    def help(self) -> str:\n        return __doc__\n",
    CLASS_DOCSTRING_REPORT ;
    "method_reads_module_doc"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    fetch = lambda: 1\n",
    CLASS_DOCSTRING_REPORT ;
    "lambda_reads_nothing"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    def run(self, text=__doc__):\n        return text\n",
    common::PASSED ;
    "method_default_reads_it"
)]
#[test_case(
    "def show(text):\n    return lambda func: func\n\n\nclass Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    @\
        show(__doc__)\n    def run(self):\n        return 1\n",
    common::PASSED ;
    "decorator_reads_it"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    run = staticmethod(lambda text=__doc__: text)\n",
    common::PASSED ;
    "lambda_default_reads_it"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    class Inner(dict, doc=__doc__):\n        pass\n",
    common::PASSED ;
    "nested_class_header_reads_it"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    fetch = lambda: __doc__\n",
    CLASS_DOCSTRING_REPORT ;
    "lambda_body_reads_module_doc"
)]
#[test_case(
    "class Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    class Inner:\n        help = __doc__\n",
    CLASS_DOCSTRING_REPORT ;
    "nested_class_body_reads_module_doc"
)]
#[test_case(
    "def show(text):\n    return lambda func: func\n\n\nclass Sync:\n    \"\"\"Synchronize the mirror.\"\"\"\n\n    @\
        show(__doc__)\n    class Inner:\n        pass\n",
    common::PASSED ;
    "nested_class_decorator_reads_it"
)]
fn class_body_reading_doc_keeps_its_docstring(source: &str, stdout: &str) {
    let root = common::project(&[("pyproject.toml", ""), ("module.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP901"]).1, stdout);
}

const CLASS_DOCSTRING_REPORT: &str = "module.py:2:5: SMP901 Class docstring on internal code; let the name, signature \
    and types document it\nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n";

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
                "def _run() -> None:\n{indent}\"\"\"Retry once, as the first call after a restart times out.\"\"\"\n\
                    {indent}print()\n"
            ),
        ),
    ]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP901"]);
    assert_eq!(
        common::read(root.path(), "module.py"),
        format!(
            "def _run() -> None:\n{indent}# Retry once, as the first\n{indent}# call after a restart\n{indent}# times \
                out.\n{indent}print()\n"
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
            "def test_probe() -> None:\n    \"\"\"Check that adding one more test keeps the run stable.\"\"\"\n    \
                assert True\n",
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
    "from attrs import define\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param x: Where it sits.\n    \"\"\"\
        \n\n    _x: int\n",
    common::PASSED ;
    "attrs_without_underscores"
)]
#[test_case(
    "from attrs import define, field\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param pos: Where it sits.\
        \n    \"\"\"\n\n    _x: int = field(alias=\"pos\")\n",
    common::PASSED ;
    "attrs_alias"
)]
#[test_case(
    "from dataclasses import dataclass\n\n@dataclass\nclass Point:\n    \"\"\"A point.\n\n    :param _x: Where it sits.\
        \n    \"\"\"\n\n    _x: int\n",
    common::PASSED ;
    "dataclass_keeps_name"
)]
#[test_case(
    "from attrs import define, field\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param x: Where it sits.\n    \
        \"\"\"\n\n    _x: int = field(alias=\"pos\")\n",
    "model.py:7:6: SMP917 `:param x:` documents `x`, which `Point` does not take\nFound 1 error.\n1 unsafe fix \
        available with `--fix --unsafe-fixes`.\n" ;
    "attrs_alias_replaces_name"
)]
#[test_case(
    "from attrs import define, field\n\nPOS = \"pos\"\n\n\n@define\nclass Point:\n    \"\"\"A point.\n\n    :param pos\
        : Where it sits.\n    \"\"\"\n\n    _x: int = field(alias=POS)\n",
    common::PASSED ;
    "attrs_alias_not_a_literal_is_unknown"
)]
fn constructor_parameters_follow_the_generator(source: &str, stdout: &str) {
    let root =
        common::project(&[("pyproject.toml", "[tool.simplint]\ndocstring-style = \"rst\"\n"), ("model.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP916,SMP917"]).1, stdout);
}

const DUPLICATED: &str = r#"class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str) -> None:
        """Set up.

        :param source: The path to read.
        """
"#;
const KEPT_CLASS: &str = r"model.py:10:10: SMP918 `:param source:` repeats the docstring of `Reader`, which autodoc shows; document each constructor parameter once
Found 1 error.
1 unsafe fix available with `--fix --unsafe-fixes`.
";
const KEPT_INIT: &str = r"model.py:4:6: SMP918 `:param source:` repeats the docstring of `__init__`, which autodoc shows; document each constructor parameter once
Found 1 error.
1 unsafe fix available with `--fix --unsafe-fixes`.
";
const RST: &str = "[tool.simplint]\ndocstring-style = \"rst\"\n";
const INDEX: &str = "Index\n=====\n";
const INIT: &str = "autoclass_content = \"init\"\n";

// The concise report of `model.py` in a project made of `RST`, the model and the given files, which replace the
// project's own when they share a name.
fn duplicate_report(files: &[(&str, &str)]) -> String {
    let mut all = vec![("pyproject.toml", RST), ("model.py", DUPLICATED)];
    all.extend_from_slice(files);
    let root = common::project(&all);
    common::check(root.path(), &["--select", "SMP918"]).1
}

// Sphinx's `autoclass_content` picks the docstring it renders: under `init` the `__init__` docstring alone, so the class
// copy is the repeat. A value the rule cannot read leaves both copies alone.
#[test_case("", KEPT_CLASS ; "unset_keeps_the_class_docstring")]
#[test_case("autoclass_content = \"class\"\n", KEPT_CLASS ; "class_keeps_the_class_docstring")]
#[test_case("autoclass_content = \"both\"\n", KEPT_CLASS ; "both_keeps_the_class_docstring")]
#[test_case(INIT, KEPT_INIT ; "init_reports_the_class_docstring")]
#[test_case("autoclass_content: str = \"init\"\n", KEPT_INIT ; "annotated_init")]
#[test_case("autoclass_content, autodoc_typehints = \"init\", \"none\"\n", KEPT_INIT ; "tuple_assignment")]
#[test_case("autoclass_content = \"init\"\nautoclass_content = \"class\"\n", KEPT_CLASS ; "last_assignment_decides")]
#[test_case("project, version = \"x\", \"1\"\nautoclass_content = \"init\"\n", KEPT_INIT ; "unrelated_tuple_is_ignored")]
#[test_case("if True:\n    VERSION = 1\nautoclass_content = \"init\"\n", KEPT_INIT ; "unrelated_block_is_ignored")]
fn autoclass_content_selects_the_copy(conf: &str, stdout: &str) {
    assert_eq!(duplicate_report(&[("docs/index.rst", INDEX), ("docs/conf.py", conf)]), stdout);
}

// `autodoc_default_options` sets `class-doc-from` for every class and overrides `autoclass_content`; a key it does not
// set leaves `autoclass_content` in charge.
#[test_case("autodoc_default_options = {\"class-doc-from\": \"init\"}\n", KEPT_INIT ; "default_options")]
#[test_case(
    "autoclass_content = \"both\"\nautodoc_default_options = {\"class-doc-from\": \"init\"}\n",
    KEPT_INIT ;
    "default_options_override_autoclass_content"
)]
#[test_case(
    "autodoc_default_options = {\"members\": True}\nautoclass_content = \"init\"\n",
    KEPT_INIT ;
    "default_options_without_the_key"
)]
#[test_case(
    "autodoc_default_options = {\"class-doc-from\": \"class\"}\nautoclass_content = \"init\"\n",
    KEPT_CLASS ;
    "default_options_class_overrides"
)]
fn autodoc_default_options_selects_the_copy(conf: &str, stdout: &str) {
    assert_eq!(duplicate_report(&[("docs/index.rst", INDEX), ("docs/conf.py", conf)]), stdout);
}

// A `class-doc-from` option on an `autoclass` directive decides the copy of the class it names, ahead of the options
// of the configuration.
#[test_case("", ".. autoclass:: Reader\n    :class-doc-from: init\n", KEPT_INIT ; "directive_alone")]
#[test_case(
    INIT,
    ".. autoclass:: Reader\n    :class-doc-from: class\n",
    KEPT_CLASS ;
    "directive_overrides_autoclass_content"
)]
#[test_case(
    "autodoc_default_options = {\"class-doc-from\": \"init\"}\n",
    ".. autoclass:: Reader\n    :class-doc-from: both\n",
    KEPT_CLASS ;
    "directive_overrides_default_options"
)]
#[test_case(
    INIT,
    ".. autoclass:: Other\n    :class-doc-from: class\n",
    KEPT_INIT ;
    "directive_for_another_class_is_not_used"
)]
#[test_case(INIT, ".. autoclass:: Reader\n    :class-doc-from: sometimes\n", common::PASSED ; "unknown_directive_value")]
fn class_doc_from_on_a_directive_selects_the_copy(conf: &str, api: &str, stdout: &str) {
    let files = [("docs/index.rst", INDEX), ("docs/conf.py", conf), ("docs/api.rst", api)];
    assert_eq!(duplicate_report(&files), stdout);
}

// A module of a package is named by its dotted path from the package's root, so a directive that names it in full
// selects its class.
#[test]
fn packaged_module_is_named_by_its_dotted_path() {
    let api = ".. autoclass:: pkg.model.Reader\n    :class-doc-from: class\n";
    let root = common::project(&[
        ("pyproject.toml", RST),
        ("pkg/__init__.py", ""),
        ("pkg/model.py", DUPLICATED),
        ("docs/index.rst", INDEX),
        ("docs/conf.py", INIT),
        ("docs/api.rst", api),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP918"]).1;
    assert!(stdout.contains(&format!("{}:10:10: SMP918", common::native("pkg/model.py"))), "{stdout}");
}

// A `conf.py` is a Sphinx configuration beside an index file, or when it names the extensions or master document.
#[test_case("docs/index.rst", INDEX, INIT, KEPT_INIT ; "index_rst")]
#[test_case("docs/index.md", INDEX, INIT, KEPT_INIT ; "index_md")]
#[test_case("docs/index.txt", INDEX, INIT, KEPT_INIT ; "index_txt")]
#[test_case("", "", "extensions = []\nautoclass_content = \"init\"\n", KEPT_INIT ; "names_extensions")]
#[test_case("", "", "master_doc = \"start\"\nautoclass_content = \"init\"\n", KEPT_INIT ; "names_master_doc")]
#[test_case("", "", "root_doc = \"start\"\nautoclass_content = \"init\"\n", KEPT_INIT ; "names_root_doc")]
#[test_case("", "", INIT, KEPT_CLASS ; "neither_index_nor_name_is_not_sphinx")]
fn sphinx_configuration_is_one_beside_an_index_or_naming_its_master(
    index: &str,
    index_text: &str,
    conf: &str,
    stdout: &str,
) {
    let mut files = vec![("docs/conf.py", conf)];
    if !index.is_empty() {
        files.push((index, index_text));
    }
    assert_eq!(duplicate_report(&files), stdout);
}

// The forms the scan cannot read leave the docstring unknown, so no copy is removed.
#[test_case("autoclass_content = \"all\"\n", common::PASSED ; "unknown_value")]
#[test_case("autoclass_content = os.environ[\"SPHINX\"]\n", common::PASSED ; "computed_value")]
#[test_case("autoclass_content = default = \"init\"\n", common::PASSED ; "chained_assignment")]
#[test_case("autoclass_content, autodoc_typehints = load()\n", common::PASSED ; "tuple_from_a_call")]
#[test_case("autoclass_content, autodoc_typehints = \"init\"\n", common::PASSED ; "tuple_of_unequal_length")]
#[test_case("if True:\n    autoclass_content = \"init\"\n", common::PASSED ; "nested_in_if")]
#[test_case("try:\n    autoclass_content = \"init\"\nexcept ImportError:\n    pass\n", common::PASSED ; "nested_in_try")]
#[test_case("with open(\"settings\") as autoclass_content:\n    pass\n", common::PASSED ; "bound_by_with")]
#[test_case("from shared import *\n", common::PASSED ; "star_import")]
#[test_case("from shared import autoclass_content\n", common::PASSED ; "imported_name")]
#[test_case("exec(\"autoclass_content = 'init'\")\n", common::PASSED ; "exec")]
#[test_case("globals()[\"autoclass_content\"] = \"init\"\n", common::PASSED ; "globals_store")]
#[test_case("autodoc_default_options[\"class-doc-from\"] = \"init\"\n", common::PASSED ; "default_options_key_set")]
#[test_case("autodoc_default_options = {**shared}\n", common::PASSED ; "default_options_unpacked")]
#[test_case("autodoc_default_options = {name: \"init\"}\n", common::PASSED ; "default_options_computed_key")]
#[test_case("autodoc_default_options = {\"class-doc-from\": \"sometimes\"}\n", common::PASSED ; "default_options_unknown_value")]
#[test_case("autodoc_default_options = dict(class_doc_from=\"init\")\n", common::PASSED ; "default_options_from_a_call")]
#[test_case("match autoclass_content:\n    case _:\n        pass\n", common::PASSED ; "bound_by_match")]
#[test_case("try:\n    pass\nexcept ImportError as autoclass_content:\n    pass\n", common::PASSED ; "bound_by_except")]
#[test_case("import autoclass_content.sub\n", common::PASSED ; "bound_by_dotted_import")]
#[test_case("import shared as autoclass_content\n", common::PASSED ; "bound_by_import_alias")]
#[test_case("def autoclass_content():\n    pass\n", common::PASSED ; "bound_by_def")]
#[test_case("class autodoc_default_options:\n    pass\n", common::PASSED ; "bound_by_class")]
#[test_case("(autoclass_content := \"init\")\n", common::PASSED ; "bound_by_walrus")]
#[test_case("autoclass_content += \"init\"\n", common::PASSED ; "augmented_assignment")]
#[test_case("autoclass_content: str\n", common::PASSED ; "declaration_only")]
#[test_case("del autoclass_content\n", common::PASSED ; "deleted")]
#[test_case("del autodoc_default_options[\"class-doc-from\"]\n", common::PASSED ; "default_options_key_deleted")]
#[test_case("autodoc_default_options.update({\"class-doc-from\": \"init\"})\n", common::PASSED ; "default_options_method_call")]
#[test_case("autodoc_default_options[\"class-doc-from\"], x = \"init\", 1\n", common::PASSED ; "key_set_in_a_tuple")]
#[test_case("autoclass_content, x = *values, 1\n", common::PASSED ; "starred_tuple_value")]
fn forms_the_scan_cannot_read_leave_the_copies_alone(conf: &str, stdout: &str) {
    assert_eq!(duplicate_report(&[("docs/index.rst", INDEX), ("docs/conf.py", conf)]), stdout);
}

#[test]
fn unparsable_conf_py_reports_nothing() {
    let (stdout, stderr) = {
        let root = common::project(&[
            ("pyproject.toml", RST),
            ("docs/index.rst", INDEX),
            ("docs/conf.py", "autoclass_content = (\n"),
            ("model.py", DUPLICATED),
        ]);
        let (_, stdout, stderr) = common::check(root.path(), &["--select", "SMP918"]);
        (stdout, stderr)
    };
    assert_eq!((stdout.as_str(), stderr.contains("does not parse")), (common::PASSED, true));
}

// A `conf.py` that cannot be read may set the option, so nothing is reported until it reads, and then the copy follows
// the option it sets.
#[test]
fn unreadable_conf_py_reports_nothing_until_it_is_readable() {
    let root = common::project(&[("pyproject.toml", RST), ("model.py", DUPLICATED), ("docs/index.rst", INDEX)]);
    std::fs::write(root.path().join("docs/conf.py"), [0xff, b'\n']).expect("write conf.py");
    let unreadable = common::check(root.path(), &["--select", "SMP918"]).1;
    std::fs::write(root.path().join("docs/conf.py"), INIT).expect("write conf.py");
    let readable = common::check(root.path(), &["--select", "SMP918"]).1;
    assert_eq!((unreadable.contains("SMP918"), readable.as_str()), (false, KEPT_INIT));
}

// Past the size limit the file is not read, so its option is unknown.
#[test]
fn oversized_conf_py_reports_nothing() {
    let conf = format!("{INIT}{}", "#".repeat((8 << 20) + 1));
    assert_eq!(duplicate_report(&[("docs/index.rst", INDEX), ("docs/conf.py", conf.as_str())]), common::PASSED);
}

// Sphinx reads a `conf.py` Ruff excludes from linting, so the exclusion does not hide its option.
#[test]
fn ruff_excluded_conf_py_still_counts() {
    let pyproject = "[tool.simplint]\ndocstring-style = \"rst\"\n\n[tool.ruff]\nextend-exclude = [\"docs/conf.py\"]\n";
    let files = [("pyproject.toml", pyproject), ("docs/index.rst", INDEX), ("docs/conf.py", INIT)];
    assert_eq!(duplicate_report(&files), KEPT_INIT);
}

// A symlinked `conf.py` counts when its target lies inside the project, and not when it points out of it.
#[cfg(unix)]
#[test]
fn symlinked_conf_py_counts_only_inside_the_project() {
    let root = common::project(&[
        ("pyproject.toml", RST),
        ("model.py", DUPLICATED),
        ("docs/index.rst", INDEX),
        ("shared/real_conf.py", INIT),
    ]);
    std::os::unix::fs::symlink("../shared/real_conf.py", root.path().join("docs/conf.py")).expect("link");
    assert_eq!(common::check(root.path(), &["--select", "SMP918"]).1, KEPT_INIT);
}

#[cfg(unix)]
#[test]
fn symlinked_conf_py_outside_the_project_is_not_read() {
    let outside = common::project(&[("real_conf.py", INIT)]);
    let root = common::project(&[("pyproject.toml", RST), ("model.py", DUPLICATED), ("docs/index.rst", INDEX)]);
    std::os::unix::fs::symlink(outside.path().join("real_conf.py"), root.path().join("docs/conf.py")).expect("link");
    assert_eq!(common::check(root.path(), &["--select", "SMP918"]).1, KEPT_CLASS);
}

// A `conf.py` of vendored code or of another project below the root is not the project's, and a gitignored one is not
// read at all.
#[test_case(
    &[("_vendor/lib/docs/index.rst", INDEX), ("_vendor/lib/docs/conf.py", INIT)] ;
    "vendored_directory"
)]
#[test_case(
    &[("subproject/pyproject.toml", RST), ("subproject/docs/index.rst", INDEX), ("subproject/docs/conf.py", INIT)] ;
    "other_project_below_the_root"
)]
#[test_case(
    &[
        (".git/HEAD", "ref: refs/heads/main\n"),
        (".gitignore", "vendor/\n"),
        ("vendor/docs/index.rst", INDEX),
        ("vendor/docs/conf.py", INIT),
    ] ;
    "gitignored"
)]
fn conf_py_of_vendored_or_other_projects_is_not_read(files: &[(&str, &str)]) {
    assert_eq!(duplicate_report(files), KEPT_CLASS);
}

// Several Sphinx configurations of one project must agree on the option, and one that leaves it unset renders the
// class docstring, so it disagrees with one that sets `init`.
#[test_case(INIT, INIT, KEPT_INIT ; "agreeing")]
#[test_case(INIT, "autoclass_content = \"class\"\n", common::PASSED ; "disagreeing")]
#[test_case(INIT, "", common::PASSED ; "one_leaves_it_unset")]
fn sphinx_configurations_of_a_project_must_agree(first: &str, second: &str, stdout: &str) {
    let files =
        [("docs/index.rst", INDEX), ("docs/conf.py", first), ("guide/index.rst", INDEX), ("guide/conf.py", second)];
    assert_eq!(duplicate_report(&files), stdout);
}

// Each project reads its own configuration, so one project's option does not reach another that sets none.
#[test]
fn each_project_reads_its_own_configuration() {
    let root = common::project(&[
        ("a/pyproject.toml", RST),
        ("a/model.py", DUPLICATED),
        ("a/docs/index.rst", INDEX),
        ("a/docs/conf.py", INIT),
        ("b/pyproject.toml", RST),
        ("b/model.py", DUPLICATED),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP918"]).1;
    assert!(stdout.contains("a/model.py:4:6: SMP918") && stdout.contains("b/model.py:10:10: SMP918"), "{stdout}");
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
            "from attrs import define\n\n__all__ = [\"Point\"]\n\n\n@define\nclass Point:\n    \"\"\"A point on the \
                horizontal axis.\"\"\"\n\n    _x: int\n",
        ),
    ]);
    assert_eq!(
        common::check(root.path(), &["--select", "SMP916"]).1,
        "model.py:8:5: SMP916 Docstring of `Point` does not document parameter `x`; add `:param x:`\nFound 1 error.\n"
    );
}

// A placeholder body, as in an abstract method, says nothing of what it returns or yields, so its docstring is not
// asked for the fields of a value it has not written. A body with code is held to the same rules.
#[test_case(
    "from collections.abc import Iterator\n\n\ndef rows() -> Iterator[str]:\n    \"\"\"Read the rows.\n\n    :yields: \
        Each row.\n    \"\"\"\n    ...\n",
    common::PASSED ;
    "placeholder_yields"
)]
#[test_case(
    "def rows():\n    \"\"\"Read the rows.\n\n    :yields: Each row.\n    \"\"\"\n    raise NotImplementedError\n",
    common::PASSED ;
    "placeholder_raises"
)]
#[test_case(
    "def total() -> int:\n    \"\"\"Add the values.\n    \"\"\"\n    return 1\n",
    "model.py:2:5: SMP923 `total` returns a value, yet its docstring has no `:returns:` field\nFound 1 error.\n" ;
    "computed_value"
)]
#[test_case(
    "def rows() -> list[str]:\n    \"\"\"Read the rows.\n\n    :yields: Each row.\n    \"\"\"\n    return []\n",
    "model.py:2:5: SMP923 `rows` returns a value, yet its docstring has no `:returns:` field\nmodel.py:4:6: SMP923 \
        `rows` is not a generator, yet its docstring has a `:yields:` field\nFound 2 errors.\n1 unsafe fix available with `\
        --fix --unsafe-fixes`.\n" ;
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

// The members of a class include those in its `if` blocks and the loop variables of its body, so a field documents
// them.
#[test_case(
    "import sys\n\n\nclass Engine:\n    \"\"\"Run jobs.\n\n    :ivar speed: Units per second.\n    :ivar run: The \
        entry point.\n    :ivar size: How many jobs wait.\n    \"\"\"\n\n    if sys.version_info >= (3, 12):\n\n        @\
        property\n        def speed(self) -> int:\n            return 2\n\n    else:\n\n        def run(self) -> None:\
        \n            pass\n\n    for size in (1, 2):\n        pass\n",
    common::PASSED ;
    "members_in_blocks_and_loops"
)]
#[test_case(
    "class Engine:\n    \"\"\"Run jobs.\n\n    :ivar missing: Not a member.\n    \"\"\"\n\n    if True:\n        speed \
        = 1\n\n    for size in (1, 2):\n        pass\n",
    "model.py:4:6: SMP921 `:ivar missing:` documents `missing`, which `Engine` does not have\nFound 1 error.\n1 unsafe \
        fix available with `--fix --unsafe-fixes`.\n" ;
    "unknown_member_still_reported"
)]
fn class_members_in_blocks_are_documented(source: &str, stdout: &str) {
    let root =
        common::project(&[("pyproject.toml", "[tool.simplint]\ndocstring-style = \"rst\"\n"), ("model.py", source)]);
    assert_eq!(common::check(root.path(), &["--select", "SMP921"]).1, stdout);
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
    assert_eq!(common::check(root.path(), &["--select", "SMP91,SMP92"]).1, common::PASSED);
}

// Sphinx reads `kwarg` and `kwparam` as spellings of `param`, and `paramtype` as one of `type`, so the fields document
// their parameter and are not reported as unknown.
#[test_case(
    "def load(path: str, mode: str) -> None:\n    \"\"\"Load.\n\n    :param path: Where.\n    :kwarg mode: How.\n    \"\
        \"\"\n    print(path, mode)\n",
    "model.py:5:6: SMP910 Field `:kwarg mode:` uses the alias `kwarg`; write `param`\nFound 1 error.\n1 unsafe fix \
        available with `--fix --unsafe-fixes`.\n" ;
    "kwarg"
)]
#[test_case(
    "def load(mode: str) -> None:\n    \"\"\"Load.\n\n    :param mode: How.\n    :paramtype mode: str\n    \"\"\"\n    \
        print(mode)\n",
    "model.py:5:6: SMP910 Field `:paramtype mode:` uses the alias `paramtype`; write `type`\nFound 1 error.\n1 unsafe \
        fix available with `--fix --unsafe-fixes`.\n" ;
    "paramtype"
)]
#[test_case(
    "def load(path: str, mode: str) -> None:\n    \"\"\"Load.\n\n    :param path: Where.\n    :kwargs mode: How.\n    \
        \"\"\"\n    print(path, mode)\n",
    "model.py:2:5: SMP916 Docstring of `load` does not document parameter `mode`; add `:param mode:`\nmodel.py:5:6: \
        SMP910 Unknown field `:kwargs mode:`; Sphinx reads `param`, `type`, `raises`, `returns`, `rtype`, `var`, \
        `vartype` and their aliases\nFound 2 errors.\n" ;
    "misspelled"
)]
fn sphinx_parameter_aliases_document_their_parameter(source: &str, stdout: &str) {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"demo\"\n\n[tool.simplint]\nproject-kind = \"library\"\ndocstring-style = \"rst\"\n",
        ),
        ("model.py", source),
    ]);
    assert_eq!(common::check(root.path(), &["--select", "SMP910,SMP916"]).1, stdout);
}

// Dropping the type of a parameter keeps the stars that mark `*args` and `**kwargs`, which the name is written with.
#[test_case(
    "def call(*args: int, **kwargs: str) -> None:\n    r\"\"\"Call it.\n\n    :param int \\*args: The numbers.\n    :\
        param str \\*\\*kwargs: The options.\n    \"\"\"\n",
    "def call(*args: int, **kwargs: str) -> None:\n    r\"\"\"Call it.\n\n    :param \\*args: The numbers.\n    :param \
        \\*\\*kwargs: The options.\n    \"\"\"\n" ;
    "star_parameters"
)]
#[test_case(
    "def call(count: int) -> None:\n    \"\"\"Call it.\n\n    :param int count: The number.\n    \"\"\"\n",
    "def call(count: int) -> None:\n    \"\"\"Call it.\n\n    :param count: The number.\n    \"\"\"\n" ;
    "plain_parameter"
)]
fn type_field_fix_keeps_the_star_markers(source: &str, fixed: &str) {
    let root =
        common::project(&[("pyproject.toml", "[tool.simplint]\ndocstring-style = \"rst\"\n"), ("demo.py", source)]);
    common::check(root.path(), &["--fix", "--unsafe-fixes", "--select", "SMP912"]);
    assert_eq!(common::read(root.path(), "demo.py"), fixed);
}

// Renaming an alias changes the docstring, which code may read through `__doc__`, so it is an unsafe fix like the
// rule's others.
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
            "demo.py:4:6: SMP910 Field `:arg path:` uses the alias `arg`; write `param`\nFound 1 error.\n1 unsafe fix \
                available with `--fix --unsafe-fixes`.\n"
                .to_owned(),
            source.to_owned()
        )
    );
}

// Renaming an alias edits the field another rule's fix rewrites or removes, so one of the two waits for the next pass.
#[test_case(
    "def run(path: str) -> None:\n    \"\"\"Run.\n\n    :param path: Where.\n    :arg gone: Removed.\n    \"\"\"\n    \
        print(path)\n",
    "def run(path: str) -> None:\n    \"\"\"Run.\n\n    :param path: Where.\n    \"\"\"\n    print(path)\n" ;
    "removed_field"
)]
#[test_case(
    "def run(count: int):\n    \"\"\"Run.\n\n    :param count: How far.\n    :return: Each number.\n    \"\"\"\n    \
        yield from range(count)\n",
    "def run(count: int):\n    \"\"\"Run.\n\n    :param count: How far.\n    :yields: Each number.\n    \"\"\"\n    \
        yield from range(count)\n" ;
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

// Ruff's D101, D102, D103 and D106 report the undocumented public definitions of their kind, which SMP903 leaves to
// them while the project's Ruff configuration runs that code.
#[test_case("def f() -> None:\n    pass\n", "D103", "Public function `f`" ; "function")]
#[test_case("class C:\n    pass\n", "D101", "Public class `C`" ; "class")]
#[test_case("class C:\n    class N:\n        pass\n", "D106", "Public class `C.N`" ; "nested_class")]
#[test_case("class C:\n    def m(self) -> None:\n        pass\n", "D102", "Public method `C.m`" ; "method")]
fn undocumented_public_api_follows_ruff_pydocstyle(source: &str, code: &str, fragment: &str) {
    let case = common::RuffOverlap {
        settings: "[tool.simplint]\nproject-kind = \"library\"\n",
        select: "SMP903",
        ruff_code: code,
        files: &[("module.py", source)],
    };
    assert_eq!(common::reported_without_and_with_ruff(&case, fragment), [true, false]);
}

// Ruff's pydocstyle skips a function that `ignore-decorators` lists, so it reports no D103 for it, and SMP903 keeps it.
#[test_case("@route", "app.route", [true, true] ; "listed_decorator")]
#[test_case("@route(\"/run\")", "app.route", [true, true] ; "listed_call")]
#[test_case("@route", "app.other", [true, false] ; "unlisted_decorator")]
fn undocumented_public_api_keeps_ignore_decorator_functions(decorator: &str, listed: &str, reported: [bool; 2]) {
    let source = format!("from app import route\n\n\n{decorator}\ndef run() -> None:\n    pass\n");
    let settings = format!(
        "[tool.simplint]\nproject-kind = \"library\"\n[tool.ruff.lint.pydocstyle]\nignore-decorators = [\"{listed}\"]\n"
    );
    let case = common::RuffOverlap {
        settings: &settings,
        select: "SMP903",
        ruff_code: "D103",
        files: &[("module.py", source.as_str())],
    };
    assert_eq!(common::reported_without_and_with_ruff(&case, "Public function `run`"), reported);
}

// Ruff names a decorator in a package by its dotted path, so `ignore-decorators` also matches one imported relatively or
// defined in the same module.
#[test_case("from ._decos import skip\n\n\n@skip", "pkg._decos.skip", [true, true] ; "relative_import")]
#[test_case("def skip(func):\n    return func\n\n\n@skip", "pkg.api.skip", [true, true] ; "same_module")]
#[test_case("import pkg._decos\n\n\n@pkg._decos.skip", "pkg._decos.skip", [true, true] ; "absolute_import")]
#[test_case("@undefined", "pkg.api.undefined", [true, false] ; "unbound_not_listed")]
fn undocumented_public_api_keeps_package_ignore_decorator_functions(
    decorated: &str,
    listed: &str,
    reported: [bool; 2],
) {
    let source = format!("{decorated}\ndef run() -> None:\n    pass\n");
    let settings = format!(
        "[tool.simplint]\nproject-kind = \"library\"\n[tool.ruff.lint.pydocstyle]\nignore-decorators = [\"{listed}\"]\n"
    );
    let case = common::RuffOverlap {
        settings: &settings,
        select: "SMP903",
        ruff_code: "D103",
        files: &[
            ("pkg/__init__.py", ""),
            ("pkg/_decos.py", "def skip(func):\n    return func\n"),
            ("pkg/api.py", source.as_str()),
        ],
    };
    assert_eq!(common::reported_without_and_with_ruff(&case, "Public function `run`"), reported);
}

// Ruff's D418 and D419 report a docstring that SMP901 can only remove without a fix, when a file-level `noqa` withdraws
// the fix. SMP901 leaves those to Ruff whether or not the project selects them, and it keeps what Ruff skips.
#[test_case(
    "D419", "# ruff: noqa: E501\ndef run() -> None:\n    \"\"\"\"\"\"\n    print()\n", [false, false] ; "empty_function"
)]
#[test_case(
    "D419", "# ruff: noqa: E501\nclass Config:\n    \"\"\"\"\"\"\n    VALUE = 1\n", [false, false] ; "empty_class"
)]
#[test_case("D419", "def run() -> None:\n    \"\"\"\"\"\"\n    print()\n", [true, true] ; "fixable_empty_function")]
#[test_case(
    "D419",
    "# ruff: noqa: E501\ndef run() -> None:\n    \"\" \"\"\n    print()\n",
    [true, true] ;
    "implicit_concatenation"
)]
#[test_case(
    "D419", "# ruff: noqa: E501\ndef run() -> None:\n    \"\"\"Run.\"\"\"\n    print()\n", [true, true] ; "not_empty"
)]
#[test_case(
    "D418",
    "# ruff: noqa: E501\nfrom typing import overload\n\n\n@overload\ndef load(path: int) -> int:\n    \"\"\"Load an int\
        .\"\"\"\n\n\n@overload\ndef load(path: str) -> str: ...\ndef load(path):\n    return path\n",
    [false, false] ;
    "overload"
)]
#[test_case(
    "D418",
    "from typing import overload\n\n\n@overload\ndef load(path: int) -> int:\n    \"\"\"Load an int.\"\"\"\n\n\n@\
        overload\ndef load(path: str) -> str: ...\ndef load(path):\n    return path\n",
    [true, true] ;
    "fixable_overload"
)]
#[test_case(
    "D419",
    "# ruff: noqa: E501\nfrom typing import overload\n\n\n@overload\ndef load(path: int) -> int:\n    \"\"\"\"\"\"\n\n\
        \n@overload\ndef load(path: str) -> str: ...\ndef load(path):\n    return path\n",
    [false, false] ;
    "empty_overload"
)]
#[test_case(
    "D418",
    "# ruff: noqa: E501\ndef run() -> None:\n    \"\"\"Run it.\"\"\"\n    print()\n",
    [true, true] ;
    "not_overload"
)]
#[test_case(
    "D419",
    "# ruff: noqa: E501\nimport pytest\n\n\n@pytest.fixture\ndef client() -> None:\n    \"\"\"\"\"\"\n    print()\n",
    [false, false] ;
    "decorator_not_listed"
)]
fn function_docstring_is_left_to_ruff_d418_d419(ruff_code: &str, source: &str, reported: [bool; 2]) {
    let case = common::RuffOverlap { settings: "", select: "SMP901", ruff_code, files: &[("module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, "docstring on internal code"), reported);
}

// A decorator that `ignore-decorators` lists makes Ruff skip the function, so SMP901 keeps its docstring.
#[test]
fn function_docstring_keeps_ignored_decorator_docstring() {
    let case = common::RuffOverlap {
        settings: "[tool.ruff.lint.pydocstyle]\nignore-decorators = [\"pytest.fixture\"]\n",
        select: "SMP901",
        ruff_code: "D419",
        files: &[(
            "module.py",
            "# ruff: noqa: E501\nimport pytest\n\n\n@pytest.fixture\ndef client() -> None:\n    \"\"\"\"\"\"\n    print\
                ()\n",
        )],
    };
    assert_eq!(common::reported_without_and_with_ruff(&case, "docstring on internal code"), [true, true]);
}

// Ruff's D418 does not report a stub, so SMP901 keeps the overload docstring of a stub read from standard input.
#[test]
fn function_docstring_keeps_overload_docstring_in_a_stub() {
    let root = common::project(&[("pyproject.toml", "[tool.ruff.lint]\nselect = [\"D418\"]\n")]);
    let source = "# ruff: noqa: E501\nfrom typing import overload\n\n\n@overload\ndef load(path: int) -> int:\n    \"\"\
        \"Load an int.\"\"\"\n";
    let (_, stdout, _) =
        common::check_with_stdin(root.path(), &["--select", "SMP901", "--stdin-filename", "stub.pyi", "-"], source);
    assert!(stdout.contains("docstring on internal code"), "{stdout}");
}

// Ruff's D419 reports an empty docstring of a single string, which SMP902 can remove only with its fix. A file-level
// `noqa` withdraws that fix, and then SMP902 leaves the docstring to Ruff whether or not the project selects D419.
#[test_case("# ruff: noqa: E501\n\"\"\"\"\"\"\n\nimport os\n", [false, false] ; "empty_with_file_noqa")]
#[test_case("\"\"\"\"\"\"\n\nimport os\n", [true, true] ; "fixable_empty")]
#[test_case("# ruff: noqa: E501\n\"\" \"\"\n\nimport os\n", [true, true] ; "implicit_concatenation")]
#[test_case("# ruff: noqa: E501\n\"\"\"\\x20\"\"\"\n\nimport os\n", [true, true] ; "escape_is_text")]
#[test_case("# ruff: noqa: E501\n\"\"\"Tests.\"\"\"\n\nimport os\n", [true, true] ; "not_empty")]
fn test_module_docstring_is_left_to_ruff_d419(source: &str, reported: [bool; 2]) {
    let case =
        common::RuffOverlap { settings: "", select: "SMP902", ruff_code: "D419", files: &[("test_module.py", source)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, "Test module carries a docstring"), reported);
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
            "class XDGMixin:\n    @property\n    def user_data_dir(self) -> str:\n        \"\"\"Data directory.\"\"\"\
                \n        \
             return \"/data\"\n\n    def site_dir(self) -> str:\n        return \"/site\"\n\n    def ensure(self, path\
             : \
             str) -> str:\n        \"\"\"Make it.\"\"\"\n        return path\n",
        ),
        (
            "pkg/unix.py",
            "from ._xdg import XDGMixin\n\n__all__ = [\"Unix\"]\n\n\nclass Unix(XDGMixin):\n    \"\"\"Unix.\"\"\"\n\
                \n    \
             def ensure(self, path: str) -> str:\n        \"\"\"Unix override.\"\"\"\n        return path\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP901,SMP903"]);
    assert_eq!(
        stdout,
        common::native(
            "pkg/_xdg.py:7:9: SMP903 Public method `XDGMixin.site_dir` has no docstring; document it for its users\n\
             pkg/_xdg.py:11:9: SMP901 Function docstring on internal code; let the name, signature and types document \
             it\n\
             Found 2 errors.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}
