use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use test_case::test_case;

use crate::common;

#[test_case("definition-before-use", "definition_before_use.py", None ; "definition_before_use")]
#[test_case(
    "definition-before-use", "definition_before_use_floating.py", None ; "definition_before_use_floating_comment"
)]
#[test_case(
    "definition-before-use", "definition_before_use_nonl.py", None ; "definition_before_use_without_trailing_newline"
)]
#[test_case(
    "definition-before-use", "definition_before_use_classbody.py", None
    ; "definition_before_use_class_body_reads_module_name"
)]
#[test_case(
    "definition-before-use", "definition_before_use_annotations.py", None ; "definition_before_use_lazy_annotations"
)]
#[test_case(
    "definition-before-use", "definition_before_use_annotations.py", Some("py310") ; "\
        definition_before_use_eager_annotations"
)]
#[test_case(
    "definition-before-use", "definition_before_use_decorated.py", None ; "definition_before_use_decorated_lazy"
)]
#[test_case(
    "definition-before-use", "definition_before_use_decorated.py", Some("py310") ; "\
        definition_before_use_decorated_eager"
)]
#[test_case(
    "definition-before-use", "definition_before_use_import_time.py", None ; "definition_before_use_import_time"
)]
#[test_case(
    "definition-before-use", "definition_before_use_type_params.py", None ; "definition_before_use_type_params"
)]
#[test_case(
    "definition-before-use", "definition_before_use_receiver.py", None ; "definition_before_use_receiver"
)]
#[test_case(
    "definition-before-use", "definition_before_use_class_order.py", None
    ; "definition_before_use_class_order"
)]
#[test_case(
    "definition-before-use", "definition_before_use_later_helper.py", None
    ; "definition_before_use_later_helper"
)]
#[test_case(
    "definition-before-use", "definition_before_use_block_helper.py", None
    ; "definition_before_use_block_helper"
)]
#[test_case(
    "definition-before-use", "definition_before_use_block_method.py", None
    ; "definition_before_use_block_method"
)]
#[test_case(
    "definition-before-use", "definition_before_use_mid_import.py", None ; "definition_before_use_mid_import"
)]
#[test_case(
    "definition-before-use", "definition_before_use_leading_import.py", None
    ; "definition_before_use_leading_import"
)]
#[test_case(
    "definition-before-use", "definition_before_use_string_tail.py", None
    ; "definition_before_use_string_line_is_not_a_comment"
)]
#[test_case(
    "definition-before-use", "definition_before_use_blank_lines.py", None
    ; "definition_before_use_keeps_blank_lines_of_adjacent_statements"
)]
#[test_case(
    "class-member-order", "class_member_order_string_tail.py", None
    ; "class_member_order_string_line_is_not_a_comment"
)]
#[test_case(
    "class-member-order", "class_member_order_pattern_binding.py", None
    ; "class_member_order_pattern_and_handler_bindings"
)]
#[test_case(
    "single-user-constant", "single_user_constant_continued_string.py", None
    ; "single_user_constant_continued_string_keeps_its_value"
)]
#[test_case(
    "definition-before-use", "definition_before_use_indented_comment.py", None
    ; "definition_before_use_deeper_comment_ends_block"
)]
#[test_case(
    "class-member-order", "class_member_order_indented_comment.py", None
    ; "class_member_order_deeper_comment_ends_block"
)]
#[test_case("single-user-constant", "single_user_constant.py", None ; "single_user_constant")]
#[test_case(
    "definition-before-use", "definition_before_use_module_defaults.py", None
    ; "definition_before_use_module_defaults"
)]
#[test_case(
    "definition-before-use", "definition_before_use_class_defaults.py", None
    ; "definition_before_use_class_defaults"
)]
#[test_case("class-member-order", "class_member_order.py", None ; "class_member_order")]
#[test_case(
    "class-member-order", "class_member_order_sections.py", None ; "class_member_order_sections"
)]
#[test_case("class-member-order", "class_member_order_pins.py", None ; "class_member_order_pins")]
#[test_case("class-member-order", "class_member_order_safety.py", None ; "class_member_order_safety")]
#[test_case("class-member-order", "class_member_order_factories.py", None ; "class_member_order_factories")]
#[test_case(
    "class-member-order", "class_member_order_annotations.py", None
    ; "class_member_order_lazy_annotations"
)]
#[test_case(
    "class-member-order", "class_member_order_annotations.py", Some("py310")
    ; "class_member_order_eager_annotations"
)]
#[test_case("single-user-constant", "single_user_constant_all.py", None ; "single_user_constant_with_all")]
#[test_case("single-user-constant", "single_user_constant_dynamic.py", None ; "single_user_constant_dynamic_all")]
#[test_case("single-user-constant", "single_user_constant_spacing.py", None ; "single_user_constant_spacing")]
#[test_case("single-user-constant", "single_user_constant_source_text.py", None ; "single_user_constant_source_text")]
#[test_case("single-user-constant", "single_user_constant_debug.py", None ; "single_user_constant_debug")]
#[test_case("single-user-constant", "single_user_constant_annotated.py", None ; "single_user_constant_annotated")]
#[test_case("single-user-constant", "single_user_constant_computed.py", None ; "single_user_constant_computed_write")]
#[test_case("single-user-constant", "single_user_constant_shared.py", None ; "single_user_constant_shared_state")]
#[test_case(
    "single-user-constant", "single_user_constant_int_attribute.py", None ; "single_user_constant_int_attribute"
)]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

#[test_case(
    "[tool.ruff.lint]\npreview = true\nselect = [\"PLR0914\"]\n[tool.ruff.lint.pylint]\nmax-locals = 2\n",
    0 ;
    "enabled_limit"
)]
#[test_case("[tool.ruff.lint]\nselect = []\n[tool.ruff.lint.pylint]\nmax-locals = 2\n", 1 ; "disabled_limit")]
#[test_case(
    "[tool.ruff.lint]\npreview = true\nselect = [\"PLR0914\"]\n[tool.ruff.lint.pylint]\nmax-locals = 3\n",
    1 ;
    "room_for_local"
)]
fn single_user_constant_respects_ruff_local_limit(config: &str, reports: usize) {
    let root = common::project(&[
        ("pyproject.toml", config),
        (
            "module.py",
            "_MESSAGE = '''hello\nworld'''\n\n\ndef read() -> str:\n    first = 1\n    second = 2\n    return _MESSAGE \
                + str(first + second)\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP202"]);
    assert_eq!(stdout.matches("`_MESSAGE` is used by one function").count(), reports, "{stdout}");
}

// One fix reorders the whole body, so it rides on the first report a comment does not suppress.
#[test]
fn definition_before_use_fixes_after_first_report_is_suppressed() {
    let root = common::project(&[(
        "module.py",
        "def main():\n    return helper()\n\n\ndef late():  # noqa: SMP201\n    return 1\n\n\ndef helper():\n    \
            return late()\n",
    )]);
    common::fix_safe(root.path(), "SMP201");
    assert_eq!(
        common::read(root.path(), "module.py"),
        "def main():\n    return helper()\n\n\ndef helper():\n    return late()\n\n\ndef late():  # noqa: SMP201\n    \
            return 1\n"
    );
}

#[test]
fn single_user_constant_still_reports_inlineable_value_at_local_limit() {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[tool.ruff.lint]\npreview = true\nselect = [\"PLR0914\"]\n[tool.ruff.lint.pylint]\nmax-locals = 1\n",
        ),
        ("module.py", "_OFFSET = 1\n\n\ndef read() -> int:\n    value = 2\n    return value + _OFFSET\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP202"]);
    assert!(stdout.contains("`_OFFSET` is used by one function"), "{stdout}");
}

// SMP201 and SMP203 read one expected order, so one `--fix` run over every fixture settles them: a second run changes
// no file.
#[test]
fn class_layout_rules_settle_every_fixture_in_one_fix_run() {
    let root = common::fixture_project();
    let fix =
        ["check", "--no-cache", "--fix", "--unsafe-fixes", "--select", "SMP201,SMP203", "--target-version", "py314"];
    let before = python_sources(root.path());
    common::simplint(root.path(), &fix);
    let settled = python_sources(root.path());
    assert_ne!(settled, before);
    common::simplint(root.path(), &fix);
    assert_eq!(python_sources(root.path()), settled);
}

fn python_sources(root: &Path) -> BTreeMap<PathBuf, String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(|entry| entry.expect("project entry"))
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "py"))
        .map(|entry| (entry.path().to_path_buf(), fs::read_to_string(entry.path()).expect("source")))
        .collect()
}

// DJ012 reports the statement of a Django model that the Django Style Guide places earlier, when the file imports
// Django. The run leaves that statement to Ruff and keeps the members DJ012 does not check, and the report that carries
// the class's fix, which DJ012 lacks. No Ruff rule is selected here, so the skip does not depend on the selection.
#[test_case(DJANGO_MODEL_SAVE_BEFORE_META, 1 ; "meta_after_save_carries_fix")]
#[test_case(DJANGO_MODEL_PROPERTY_AFTER_METHOD, 1 ; "property_after_method")]
#[test_case(DJANGO_MODEL_WITHOUT_IMPORT, 1 ; "no_django_import")]
#[test_case(DJANGO_MODEL_INIT_AFTER_METHOD, 1 ; "init_after_method")]
#[test_case(DJANGO_MODEL_MANAGER_AFTER_METHOD, 1 ; "manager_after_method")]
#[test_case(DJANGO_MODEL_URL_BEFORE_PROPERTY, 1 ; "url_before_property")]
#[test_case(DJANGO_MODEL_PLAIN_ATTRIBUTE, 1 ; "plain_attribute")]
#[test_case(DJANGO_MODEL_NESTED_CLASS, 1 ; "nested_class")]
#[test_case(DJANGO_MODEL_DOCSTRING_BEFORE_META, 1 ; "docstring_before_meta")]
#[test_case(DJANGO_MODEL_INIT_AND_META_AFTER_METHOD, 1 ; "second_inversion_meta_left_to_ruff")]
fn class_member_order_leaves_dj012_statements_to_ruff(source: &str, reports: usize) {
    let root = common::project(&[("models.py", source)]);
    let (_, stdout, _) =
        common::simplint(root.path(), &["check", "--no-cache", "--select", "SMP203", "--output-format", "json"]);
    assert_eq!(stdout.matches("\"code\": \"SMP203\"").count(), reports, "{stdout}");
}

const DJANGO_MODEL_SAVE_BEFORE_META: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    name \
    = models.CharField(max_length=10)\n\n    def save(self, *args: object, **kwargs: object) -> None:\n        pass\n\
    \n    class Meta:\n        ordering = [\"name\"]\n";
const DJANGO_MODEL_PROPERTY_AFTER_METHOD: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    \
    name = models.CharField(max_length=10)\n\n    def helper(self) -> str:\n        return self.name\n\n    @property\
    \n    def full(self) -> str:\n        return self.name\n";
const DJANGO_MODEL_WITHOUT_IMPORT: &str = "from orm import models\n\n\nclass Article(models.Model):\n    def __str__(\
    self) -> str:\n        return \"\"\n\n    name = models.CharField(max_length=10)\n\n    def helper(self) -> None:\
    \n        pass\n\n    @property\n    def full(self) -> str:\n        return self.name\n";
const DJANGO_MODEL_INIT_AFTER_METHOD: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    def \
    helper(self) -> None:\n        pass\n\n    def __init__(self) -> None:\n        super().__init__()\n";
const DJANGO_MODEL_MANAGER_AFTER_METHOD: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    \
    def helper(self) -> None:\n        pass\n\n    objects = models.Manager()\n\n    @property\n    def full(self) -> str\
    :\n        return \"\"\n";
const DJANGO_MODEL_URL_BEFORE_PROPERTY: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    \
    def get_absolute_url(self) -> str:\n        return \"/\"\n\n    @property\n    def full(self) -> str:\n        \
    return \"\"\n";
const DJANGO_MODEL_PLAIN_ATTRIBUTE: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    def \
    helper(self) -> None:\n        pass\n\n    LIMIT = 3\n\n    @property\n    def full(self) -> str:\n        return \"\"\
    \n";
const DJANGO_MODEL_NESTED_CLASS: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    def helper\
    (self) -> None:\n        pass\n\n    class Config:\n        pass\n";
const DJANGO_MODEL_DOCSTRING_BEFORE_META: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    \
    \"\"\"An article.\"\"\"\n\n    def helper(self) -> None:\n        pass\n\n    class Meta:\n        ordering = [\"\
    name\"]\n";
const DJANGO_MODEL_INIT_AND_META_AFTER_METHOD: &str = "from django.db import models\n\n\nclass Article(models.Model):\
    \n    def helper(self) -> None:\n        pass\n\n    def __init__(self) -> None:\n        super().__init__()\n\
    \n    class Meta:\n        ordering = [\"name\"]\n";
