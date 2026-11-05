use test_case::test_case;

use crate::common;

#[test_case("dataclass-frozen", "dataclass_frozen.py", "py314" ; "dataclass_frozen")]
#[test_case("dataclass-frozen", "dataclass_frozen_receivers.py", "py314" ; "dataclass_frozen_receivers")]
#[test_case("dataclass-frozen", "dataclass_frozen_receiver_names.py", "py314" ; "dataclass_frozen_receiver_names")]
#[test_case("dataclass-slots", "dataclass_slots_receiver_names.py", "py314" ; "dataclass_slots_receiver_names")]
#[test_case("dataclass-frozen", "dataclass_frozen_runtime.py", "py314" ; "dataclass_frozen_runtime")]
#[test_case("dataclass-slots", "dataclass_slots.py", "py314" ; "dataclass_slots")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py314" ; "dataclass_kw_only")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py39" ; "dataclass_kw_only_py39")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_calls.py", "py314" ; "dataclass_kw_only_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_debug.py", "py314" ; "dataclass_kw_only_debug")]
#[test_case("indistinct-error-class", "ierr_errors.py", "py314" ; "indistinct_error_class")]
#[test_case("indistinct-error-class", "ierr_app.py", "py314" ; "indistinct_error_class_catching_module")]
#[test_case("indistinct-error-class", "ierr_reexport.py", "py314" ; "indistinct_error_class_reexport")]
#[test_case("indistinct-error-class", "test_ierr_errors.py", "py314" ; "indistinct_error_class_test_module")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
