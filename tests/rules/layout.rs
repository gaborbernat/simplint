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
#[test_case("single-user-constant", "single_user_constant_pattern.py", None ; "single_user_constant_compiled_pattern")]
#[test_case("single-user-constant", "single_user_constant_computed.py", None ; "single_user_constant_computed_write")]
#[test_case("single-user-constant", "single_user_constant_containers.py", None ; "single_user_constant_containers_are_not_reported")]
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

// The values CPython 3.11 folds at compile time, which the rule reports, and the ones it leaves to run per call. The
// cases match what 3.11.18 and 3.15.2 compile to LOAD_CONST or not.
#[test_case("(1, 2)", true ; "tuple_of_constants")]
#[test_case("((1, 2), (3, ('a', b'x')))", true ; "nested_tuples")]
#[test_case("()", true ; "empty_tuple")]
#[test_case("(-1, 1.5, 1j, None, ..., True)", true ; "mixed_constants")]
#[test_case("(1, 2) + (3,)", true ; "tuple_concatenation")]
#[test_case("(1, 2) * 3", true ; "tuple_repetition_within_limits")]
#[test_case("('a',) * 10000", false ; "tuple_repetition_beyond_collection_size")]
#[test_case("(1, 2) * 1000", false ; "tuple_repetition_beyond_collection_size_with_pairs")]
#[test_case("('a', 'b') * 2000", false ; "pairs_repeated_beyond_collection_size")]
#[test_case("1 / 0", false ; "division_by_zero")]
#[test_case("(1 / 0,)", false ; "tuple_holding_division_by_zero")]
#[test_case("'a' - 'b'", false ; "string_subtraction_is_a_type_error")]
#[test_case("(1, 'a' - 'b')", false ; "tuple_holding_type_error")]
#[test_case("2 ** 100000", false ; "power_beyond_int_size")]
#[test_case("2 ** 128", false ; "power_of_two_beyond_int_size")]
#[test_case("2 ** 64", true ; "power_within_int_size")]
#[test_case("2 ** 200", false ; "power_of_two_at_two_hundred")]
#[test_case("(2 ** 100000,)", false ; "tuple_holding_power_beyond_int_size")]
#[test_case("(\"a\", 2 ** 200)", false ; "tuple_holding_power_of_two_at_two_hundred")]
#[test_case("'a' * 5000", false ; "string_repetition_beyond_string_size")]
#[test_case("\"-\" * 4097", false ; "string_repetition_one_past_string_size")]
#[test_case("\"-\" * 4096", true ; "string_repetition_at_string_size")]
#[test_case("b'a' * 5000", false ; "bytes_repetition_beyond_string_size")]
#[test_case("'%s-%s' % (1, 2)", false ; "percent_formatting_is_not_folded")]
#[test_case("'%d' % 'x'", false ; "percent_formatting_with_a_type_error")]
#[test_case("1 << 100000", false ; "shift_beyond_int_size")]
#[test_case("1 @ 2", false ; "matrix_multiplication_is_a_type_error")]
#[test_case("60 * 60", true ; "integer_product")]
#[test_case("-1", true ; "negative_literal")]
#[test_case("-(1)", true ; "negated_parenthesised_literal")]
#[test_case("1 + 2j", true ; "complex_sum")]
#[test_case("2 ** -1", true ; "power_with_negative_exponent")]
#[test_case("0 ** -1", false ; "zero_to_negative_power")]
#[test_case("0 ** 0", true ; "zero_to_zero")]
#[test_case("1 ** 1000", false ; "power_of_one_beyond_int_size")]
#[test_case("7 // 2", true ; "floor_division")]
#[test_case("7 % 0", false ; "modulo_by_zero")]
#[test_case("7 // 0", false ; "floor_division_by_zero")]
#[test_case("1.5 // 0.5", true ; "float_floor_division")]
#[test_case("1.5 / 0", false ; "float_division_by_zero")]
#[test_case("5.0 % 0.0", false ; "float_modulo_by_zero")]
#[test_case("2.0 ** 3", true ; "float_power")]
#[test_case("1j / 2", true ; "complex_division_by_a_number")]
#[test_case("2 << -1", false ; "negative_shift")]
#[test_case("0 << 5", true ; "shift_of_zero")]
#[test_case("2 << 200", false ; "shift_beyond_int_size")]
#[test_case("256 >> 2", true ; "right_shift")]
#[test_case("1 >> -1", false ; "negative_right_shift")]
#[test_case("5 & 3", true ; "bitwise_and")]
#[test_case("-None", false ; "negated_none")]
#[test_case("-'a'", false ; "negated_string")]
#[test_case("-123456789012345678901234567890", true ; "negated_integer_beyond_u64")]
#[test_case("'a' + 'b'", true ; "string_concatenation")]
#[test_case("b'a' + b'b'", true ; "bytes_concatenation")]
#[test_case("'a' + b'b'", false ; "string_and_bytes_concatenation")]
#[test_case("'ab' % 1", false ; "string_formatting")]
#[test_case("None + 1", false ; "none_arithmetic")]
#[test_case("((1, 2),) * 100", true ; "nested_tuple_repetition_within_limits")]
#[test_case("((1, 2),) * 400", false ; "nested_tuple_repetition_beyond_collection_size")]
#[test_case("(((1, 2, 3),),) * 200", true ; "deeply_nested_tuple_repetition_within_limits")]
#[test_case("(((1, 2, 3),),) * 205", false ; "deeply_nested_tuple_repetition_beyond_total_items")]
#[test_case("(1, 2) * 0", true ; "tuple_repeated_zero_times")]
#[test_case("'' * -5", true ; "empty_string_repeated_negatively")]
#[test_case("'a' * -1", false ; "string_repeated_negatively")]
#[test_case("b'' * 10 ** 30", false ; "empty_bytes_repeated_beyond_index_size")]
#[test_case("123456789012345678901234567890 + 1", false ; "arithmetic_on_an_integer_beyond_u64_is_not_modelled")]
#[test_case("-1.5", true ; "negated_float_literal")]
#[test_case("-1j", true ; "negated_complex_literal")]
#[test_case("1 + 2.0", true ; "integer_plus_float")]
#[test_case("1j / 2.0", true ; "complex_division_by_a_float")]
#[test_case("1j / 0", false ; "complex_division_by_zero")]
#[test_case("1j % 2", false ; "complex_modulo_is_not_folded")]
#[test_case("() * 3", true ; "empty_tuple_repetition")]
#[test_case("5 - 3", true ; "integer_subtraction")]
#[test_case("(2 ** 63) * (2 ** 63) * (2 ** 63)", false ; "integer_product_beyond_int_size")]
#[test_case("1 | 2", true ; "integer_or")]
#[test_case("1 ^ 2", true ; "integer_xor")]
#[test_case("2 ** 0", true ; "power_with_zero_exponent")]
#[test_case("5 << 0", true ; "shift_by_zero")]
#[test_case("1 << 3", true ; "left_shift")]
#[test_case("-1 << 3", true ; "negative_left_shift")]
#[test_case("7 % 3", true ; "integer_modulo")]
#[test_case("-7 % 3", true ; "integer_modulo_with_negative_dividend")]
#[test_case("7 % -3", true ; "integer_modulo_with_negative_divisor")]
#[test_case("1.5 + 2.0", true ; "float_sum")]
#[test_case("1.5 - 1.0", true ; "float_difference")]
#[test_case("1.5 * 2.0", true ; "float_product")]
#[test_case("1.5 << 1", false ; "float_shift_is_not_folded")]
#[test_case("0.0 ** -1.0", false ; "zero_float_to_negative_power")]
#[test_case("7.5 % 2.0", true ; "float_modulo")]
#[test_case("-7.5 % 2.0", true ; "float_modulo_with_negative_dividend")]
#[test_case("2 * -None", false ; "right_operand_that_does_not_fold")]
#[test_case("3 * b'a'", true ; "count_times_bytes")]
#[test_case("3 * (1, 2)", true ; "count_times_tuple")]
#[test_case("1j + 'a'", false ; "complex_plus_string")]
#[test_case("(1,) * -1", false ; "tuple_repeated_negatively")]
#[test_case("(1, 2) * 2 ** 63", false ; "tuple_repeated_beyond_index_size")]
#[test_case("7 / 2", true ; "integer_true_division")]
#[test_case("1 >> 200", true ; "right_shift_beyond_int_size")]
#[test_case("-1 >> 200", true ; "negative_right_shift_beyond_int_size")]
#[test_case("0 ** 5", true ; "zero_to_positive_power")]
#[test_case("1 << 127", false ; "left_shift_beyond_i128_is_not_modelled")]
#[test_case("-7 // 2", true ; "floor_division_rounding_down")]
#[test_case("1.5 / 2.0", true ; "float_true_division")]
#[test_case("(-2.0) ** 0.5", false ; "negative_float_to_fractional_power")]
#[test_case("(-2.0) ** 2.0", true ; "negative_float_to_whole_power")]
fn single_user_constant_reports_only_what_the_compiler_folds(value: &str, reported: bool) {
    let source = format!("_VALUE = {value}\n\n\ndef read() -> object:\n    return _VALUE\n");
    let root = common::project(&[("module.py", source.as_str())]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP202"]);
    assert_eq!(stdout.contains("`_VALUE` is used by one function"), reported, "{stdout}");
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
#[test_case(DJANGO_MODEL_UNPACKED_ATTRIBUTE, 1 ; "unpacked_attribute")]
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
const DJANGO_MODEL_UNPACKED_ATTRIBUTE: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    \
    def helper(self) -> None:\n        pass\n\n    LOW, HIGH = 1, 3\n\n    @property\n    def full(self) -> str:\n        \
    return \"\"\n";
const DJANGO_MODEL_NESTED_CLASS: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    def helper\
    (self) -> None:\n        pass\n\n    class Config:\n        pass\n";
const DJANGO_MODEL_DOCSTRING_BEFORE_META: &str = "from django.db import models\n\n\nclass Article(models.Model):\n    \
    \"\"\"An article.\"\"\"\n\n    def helper(self) -> None:\n        pass\n\n    class Meta:\n        ordering = [\"\
    name\"]\n";
const DJANGO_MODEL_INIT_AND_META_AFTER_METHOD: &str = "from django.db import models\n\n\nclass Article(models.Model):\
    \n    def helper(self) -> None:\n        pass\n\n    def __init__(self) -> None:\n        super().__init__()\n\
    \n    class Meta:\n        ordering = [\"name\"]\n";
