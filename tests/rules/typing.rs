use test_case::test_case;

use crate::common;

#[test_case("missing-final", "missing_final.py", "py314" ; "missing_final")]
#[test_case("missing-final", "missing_final_typing.py", "py314" ; "missing_final_extends_typing_import")]
#[test_case("missing-final", "missing_final_sorted.py", "py314" ; "missing_final_keeps_isort_order")]
#[test_case("missing-final", "missing_final_order.py", "py314" ; "missing_final_places_new_import")]
#[test_case("missing-final", "missing_final_alias.py", "py314" ; "missing_final_uses_typing_alias")]
#[test_case("missing-final", "missing_final_module.py", "py314" ; "missing_final_typing_module")]
#[test_case("missing-final", "missing_final_new.py", "py314" ; "missing_final_new_import")]
#[test_case("missing-final", "missing_final_bare.py", "py314" ; "missing_final_import_at_top")]
#[test_case("missing-final", "missing_final_checking.py", "py310" ; "missing_final_type_checking_eager")]
#[test_case("missing-final", "missing_final_checking.py", "py314" ; "missing_final_type_checking_lazy")]
#[test_case("missing-final", "missing_final_future.py", "py310" ; "missing_final_type_checking_future")]
#[test_case("missing-final", "missing_final.py", "py37" ; "missing_final_before_python38")]
#[test_case("missing-final", "missing_final_gate.py", "py314" ; "missing_final_version_gate")]
#[test_case("missing-final", "missing_final_branch.py", "py314" ; "missing_final_version_branch")]
#[test_case("imprecise-annotation", "imprecise_annotation.py", "py314" ; "imprecise_annotation")]
#[test_case("imprecise-annotation", "imprecise_annotation_type_params.py", "py314" ; "imprecise_annotation_type_params")]
#[test_case("callable-ellipsis", "callable_ellipsis.py", "py314" ; "callable_ellipsis")]
#[test_case("vars-call", "vars_call.py", "py314" ; "vars_call")]
#[test_case("vars-call", "vars_call_bound.py", "py314" ; "vars_call_shadowed")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// Ruff's ANN401 reports an `Any` that is a whole annotation, or starts one as in `Any | None`, which SMP602 leaves to it
// while the project's Ruff configuration runs it.
#[test_case("def f(value: Any | None) -> None:\n    print(value)\n", "ANN401", "`Any` accepts any value" ; "parameter")]
#[test_case("def f(value: Any) -> None:\n    print(value)\n", "ANN401", "`Any` accepts any value" ; "whole_parameter")]
#[test_case("def f() -> Any:\n    return 1\n", "ANN401", "`Any` accepts any value" ; "whole_return")]
fn imprecise_annotation_follows_ruff_any_type(source: &str, code: &str, fragment: &str) {
    for selected in [false, true] {
        let select = if selected { format!("\"{code}\"") } else { String::new() };
        let root = common::project(&[
            ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [{select}]\n")),
            ("module.py", &format!("from typing import Any\n\n{source}")),
        ]);
        let stdout = common::check(root.path(), &["--select", "SMP602"]).1;
        assert_eq!(stdout.contains(fragment), !selected, "{stdout}");
    }
}
