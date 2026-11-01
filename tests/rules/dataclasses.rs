use test_case::test_case;

use crate::common;

#[test_case("dataclass-options", "dataclass_options.py", "py314" ; "dataclass_options")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py314" ; "dataclass_kw_only")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py39" ; "dataclass_kw_only_py39")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}
