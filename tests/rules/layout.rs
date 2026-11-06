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
#[test_case(
    "definition-before-use", "definition_before_use_type_params.py", "py314" ; "definition_before_use_type_params"
)]
#[test_case(
    "definition-before-use", "definition_before_use_receiver.py", "py314" ; "definition_before_use_receiver"
)]
#[test_case(
    "definition-before-use", "definition_before_use_class_order.py", "py314"
    ; "definition_before_use_class_order"
)]
#[test_case("single-user-constant", "single_user_constant.py", "py314" ; "single_user_constant")]
#[test_case(
    "definition-before-use", "definition_before_use_module_defaults.py", "py314"
    ; "definition_before_use_module_defaults"
)]
#[test_case(
    "definition-before-use", "definition_before_use_class_defaults.py", "py314"
    ; "definition_before_use_class_defaults"
)]
#[test_case("class-member-order", "class_member_order.py", "py314" ; "class_member_order")]
#[test_case(
    "class-member-order", "class_member_order_sections.py", "py314" ; "class_member_order_sections"
)]
#[test_case("class-member-order", "class_member_order_pins.py", "py314" ; "class_member_order_pins")]
#[test_case("class-member-order", "class_member_order_safety.py", "py314" ; "class_member_order_safety")]
#[test_case("class-member-order", "class_member_order_factories.py", "py314" ; "class_member_order_factories")]
#[test_case(
    "class-member-order", "class_member_order_annotations.py", "py314"
    ; "class_member_order_lazy_annotations"
)]
#[test_case(
    "class-member-order", "class_member_order_annotations.py", "py310"
    ; "class_member_order_eager_annotations"
)]
#[test_case("single-user-constant", "single_user_constant_all.py", "py314" ; "single_user_constant_with_all")]
#[test_case("single-user-constant", "single_user_constant_dynamic.py", "py314" ; "single_user_constant_dynamic_all")]
#[test_case("single-user-constant", "single_user_constant_spacing.py", "py314" ; "single_user_constant_spacing")]
#[test_case("single-user-constant", "single_user_constant_source_text.py", "py314" ; "single_user_constant_source_text")]
#[test_case("single-user-constant", "single_user_constant_debug.py", "py314" ; "single_user_constant_debug")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// SMP201 and SMP203 read one expected order, so one `--fix` run over every fixture settles them: a second run changes no
// file.
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

fn python_sources(root: &std::path::Path) -> std::collections::BTreeMap<std::path::PathBuf, String> {
    walkdir::WalkDir::new(root)
        .into_iter()
        .map(|entry| entry.expect("project entry"))
        .filter(|entry| entry.path().extension().is_some_and(|extension| extension == "py"))
        .map(|entry| (entry.path().to_path_buf(), std::fs::read_to_string(entry.path()).expect("source")))
        .collect()
}
