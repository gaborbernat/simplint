use test_case::test_case;

use crate::common;

#[test_case("rest-field-syntax", "rest_field_syntax.py" ; "rest_field_syntax")]
#[test_case("rest-unknown-param", "rest_unknown_param.py" ; "rest_unknown_param")]
#[test_case("rest-missing-param", "rest_missing_param.py" ; "rest_missing_param")]
#[test_case("rest-param-order", "rest_param_order.py" ; "rest_param_order")]
#[test_case("rest-redundant-type", "rest_redundant_type.py" ; "rest_redundant_type")]
#[test_case("rest-returns", "rest_returns.py" ; "rest_returns")]
#[test_case("rest-missing-raises", "rest_missing_raises.py" ; "rest_missing_raises")]
fn rule(rule: &str, fixture: &str) {
    common::assert_rule_snapshot(rule, fixture, "py314");
}

// Without `docstring-style`, the field lists of a project's docstrings are not known to be reStructuredText.
#[test]
fn rest_rules_need_the_docstring_style() {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"demo\"\n[build-system]\nrequires = []\n"),
        (
            "demo.py",
            "def run(path: str) -> str:\n    \"\"\"Run.\n\n    :parm path: Where.\n    \"\"\"\n    return path\n",
        ),
    ]);
    assert_eq!(common::check(root.path(), &["--select", "SMP3"]).1, "All checks passed!\n");
}
