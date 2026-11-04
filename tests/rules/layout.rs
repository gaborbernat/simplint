use test_case::test_case;

use crate::common;

#[test_case("definition-before-use", "definition_before_use.py", "py314" ; "definition_before_use")]
#[test_case(
    "definition-before-use", "definition_before_use_floating.py", "py314" ; "definition_before_use_floating_comment"
)]
#[test_case(
    "definition-before-use", "definition_before_use_nonl.py", "py314" ; "definition_before_use_without_trailing_newline"
)]
#[test_case(
    "definition-before-use", "definition_before_use_classbody.py", "py314"
    ; "definition_before_use_class_body_reads_module_name"
)]
#[test_case(
    "definition-before-use", "definition_before_use_annotations.py", "py314" ; "definition_before_use_lazy_annotations"
)]
#[test_case(
    "definition-before-use", "definition_before_use_annotations.py", "py310" ; "definition_before_use_eager_annotations"
)]
#[test_case(
    "definition-before-use", "definition_before_use_decorated.py", "py314" ; "definition_before_use_decorated_lazy"
)]
#[test_case(
    "definition-before-use", "definition_before_use_decorated.py", "py310" ; "definition_before_use_decorated_eager"
)]
#[test_case(
    "definition-before-use", "definition_before_use_import_time.py", "py314" ; "definition_before_use_import_time"
)]
#[test_case("single-user-constant", "single_user_constant.py", "py314" ; "single_user_constant")]
#[test_case("single-user-constant", "single_user_constant_all.py", "py314" ; "single_user_constant_with_all")]
#[test_case("single-user-constant", "single_user_constant_dynamic.py", "py314" ; "single_user_constant_dynamic_all")]
#[test_case("single-user-constant", "single_user_constant_spacing.py", "py314" ; "single_user_constant_spacing")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
