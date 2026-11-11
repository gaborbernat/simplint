use test_case::test_case;

use crate::common;

#[test_case("missing-final", "missing_final.py", "py314" ; "missing_final")]
#[test_case("missing-final", "missing_final_typing.py", "py314" ; "missing_final_extends_typing_import")]
#[test_case("missing-final", "missing_final_sorted.py", "py314" ; "missing_final_keeps_isort_order")]
#[test_case("missing-final", "missing_final_order.py", "py314" ; "missing_final_places_new_import")]
#[test_case("missing-final", "missing_final_alias.py", "py314" ; "missing_final_uses_typing_alias")]
#[test_case("missing-final", "missing_final_module.py", "py314" ; "missing_final_typing_module")]
#[test_case("missing-final", "missing_final_new.py", "py314" ; "missing_final_new_import")]
#[test_case("missing-final", "missing_final_shared_docstring.py", "py314" ; "missing_final_shared_docstring")]
#[test_case("missing-final", "missing_final_docstring_import.py", "py314" ; "missing_final_docstring_import")]
#[test_case("missing-final", "missing_final_bare.py", "py314" ; "missing_final_import_at_top")]
#[test_case("missing-final", "missing_final_checking.py", "py310" ; "missing_final_type_checking_eager")]
#[test_case("missing-final", "missing_final_checking.py", "py314" ; "missing_final_type_checking_lazy")]
#[test_case("missing-final", "missing_final_future.py", "py310" ; "missing_final_type_checking_future")]
#[test_case("missing-final", "missing_final.py", "py37" ; "missing_final_before_python38")]
#[test_case("missing-final", "missing_final_gate.py", "py314" ; "missing_final_version_gate")]
#[test_case("missing-final", "missing_final_branch.py", "py314" ; "missing_final_version_branch")]
#[test_case("missing-final", "missing_final_globals.py", "py314" ; "missing_final_globals_written")]
#[test_case("missing-final", "docs/conf.py", "py314" ; "missing_final_sphinx_conf")]
#[test_case("imprecise-annotation", "imprecise_annotation.py", "py314" ; "imprecise_annotation")]
#[test_case("imprecise-annotation", "imprecise_annotation_type_params.py", "py314" ; "imprecise_annotation_type_params")]
#[test_case("imprecise-annotation", "imprecise_annotation_ruff_overlap.py", "py314" ; "imprecise_annotation_ruff_overlap")]
#[test_case("imprecise-annotation", "imprecise_annotation_callable.py", "py314" ; "imprecise_annotation_callable")]
#[test_case("imprecise-annotation", "imprecise_annotation_annotated.py", "py314" ; "imprecise_annotation_annotated")]
#[test_case("callable-ellipsis", "callable_ellipsis.py", "py314" ; "callable_ellipsis")]
#[test_case("vars-call", "vars_call.py", "py314" ; "vars_call")]
#[test_case("vars-call", "vars_call_bound.py", "py314" ; "vars_call_shadowed")]
#[test_case("vars-call", "vars_call_mapping.py", "py314" ; "vars_call_mapping")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// A write through `globals()` under a name the code cannot read may set any name of the module, as a computed
// `setattr` does, so no name is final.
#[test_case("globals()[name] = 3" ; "computed_key")]
#[test_case("globals().update(LIMIT=3)" ; "update")]
#[test_case("globals().setdefault(name, 3)" ; "setdefault")]
#[test_case("globals().pop(name)" ; "pop")]
fn missing_final_skips_every_name_a_computed_globals_write_may_set(body: &str) {
    let root = common::project(&[("module.py", &format!("LIMIT = 1\nRATE = 2\n\n\ndef write(name):\n    {body}\n"))]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP601"]);
    assert_eq!(stdout.matches("SMP601").count(), 0, "{stdout}");
}

// A `globals` the module defines is not the builtin, so its calls write no module name and `LIMIT` stays reported.
#[test]
fn missing_final_ignores_a_globals_the_module_defines() {
    let root = common::project(&[(
        "module.py",
        "LIMIT = 1\n\n\ndef globals():\n    return {}\n\n\ndef write(name):\n    globals()[name] = 3\n",
    )]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP601"]);
    assert_eq!(stdout.matches("SMP601").count(), 1, "{stdout}");
}

// At module level `vars()` and `locals()` with no argument are the module's dict, so their writes count as `globals()`
// writes, while a call with an argument or a write in a function, class or lambda does not.
#[test_case("vars()[\"LIMIT\"] = 3", 0 ; "module_vars_store")]
#[test_case("locals()[\"LIMIT\"] = 3", 0 ; "module_locals_store")]
#[test_case("del vars()[\"LIMIT\"]", 0 ; "module_vars_delete")]
#[test_case("vars().update(LIMIT=3)", 0 ; "module_vars_update")]
#[test_case("vars(config)[\"LIMIT\"] = 3", 1 ; "vars_of_an_object")]
#[test_case("class Config:\n    vars()[\"LIMIT\"] = 3", 1 ; "class_body_vars")]
#[test_case("def write():\n    locals()[\"LIMIT\"] = 3", 1 ; "function_locals")]
#[test_case("(lambda: locals().update(LIMIT=3))()", 1 ; "lambda_locals")]
#[test_case("def write(value=vars().update(LIMIT=3)):\n    return value", 0 ; "default_of_a_parameter")]
#[test_case("@vars().update(LIMIT=3)\ndef write():\n    return None", 0 ; "decorator")]
#[test_case("class Config(vars().update(LIMIT=3) or object):\n    pass", 0 ; "class_base")]
#[test_case("(lambda value=vars().update(LIMIT=3): value)()", 0 ; "lambda_default")]
#[test_case("def write():\n    value = vars().update(LIMIT=3)", 1 ; "function_body_local")]
fn missing_final_counts_module_vars_and_locals_writes_only(body: &str, reports: usize) {
    let root = common::project(&[("module.py", &format!("LIMIT = 1\n\n\n{body}\n"))]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP601"]);
    assert_eq!(stdout.matches("SMP601").count(), reports, "{stdout}");
}

// The parenthesized name `(LIMIT)` takes no annotation of its own, as `(LIMIT: Final[int]) = 3` is not valid Python,
// but the other names of the file still get theirs.
#[test_case("(LIMIT) = 3\n", false ; "parenthesized_without_annotation")]
#[test_case("(LIMIT): int = 3\n", true ; "parenthesized_with_annotation")]
fn missing_final_fixes_a_parenthesized_name_only_when_annotated(source: &str, fixable: bool) {
    let root = common::project(&[("module.py", &format!("{source}OTHER = 4\n\nprint(LIMIT, OTHER)\n"))]);
    common::fix_unsafe(root.path(), "SMP601");
    let fixed = common::read(root.path(), "module.py");
    assert!(fixed.contains("OTHER: Final[int] = 4"), "{fixed}");
    assert_eq!(fixed.contains("(LIMIT): Final[int] = 3"), fixable, "{fixed}");
}

#[test_case("assert inspect.signature(callback)", 0 ; "inspected_not_called")]
#[test_case("assert callback(1)", 1 ; "called")]
#[test_case("assert callback.__call__(1)", 1 ; "explicit_dunder_call")]
fn callable_ellipsis_in_test_parameter_depends_on_use(body: &str, reports: usize) {
    let root = common::project(&[(
        "tests/test_callbacks.py",
        &format!(
            "from collections.abc import Callable\nimport inspect\n\n\ndef test_callback(callback: Callable[..., int]) -> None:\n    {body}\n"
        ),
    )]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP603"]);
    assert_eq!(stdout.matches("Callable[..., T]").count(), reports, "{stdout}");
}

#[test]
fn callable_ellipsis_in_test_module_outside_function_reports() {
    let root = common::project(&[(
        "tests/test_callbacks.py",
        "from collections.abc import Callable\n\ncallback: Callable[..., int]\n",
    )]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "SMP603"]);
    assert_eq!(stdout.matches("Callable[..., T]").count(), 1, "{stdout}");
}

// Ruff's ANN401 reports an `Any` that is a whole annotation, or starts one as in `Any | None`, which SMP602 leaves to it
// while the project's Ruff configuration runs it.
#[test_case("def f(value: Any | None) -> None:\n    print(value)\n", "ANN401", "`Any` accepts any value" ; "parameter")]
#[test_case("def f(value: Any) -> None:\n    print(value)\n", "ANN401", "`Any` accepts any value" ; "whole_parameter")]
#[test_case("def f() -> Any:\n    return 1\n", "ANN401", "`Any` accepts any value" ; "whole_return")]
fn imprecise_annotation_follows_ruff_any_type(source: &str, code: &str, fragment: &str) {
    let module = format!("from typing import Any\n\n{source}");
    let case =
        common::RuffOverlap { settings: "", select: "SMP602", ruff_code: code, files: &[("module.py", &module)] };
    assert_eq!(common::reported_without_and_with_ruff(&case, fragment), [true, false]);
}

// Ruff's ANN401 reports an `Any` that is, or sits in an `Optional`, a `Union`, a `|` or the first argument of
// `Annotated`, in a parameter or return annotation. SMP602 leaves those to it while the project's Ruff configuration selects it.
#[test_case("def f(value: Optional[Any]) -> None:\n    print(value)\n", false ; "optional")]
#[test_case("def f(value: Union[int, Any]) -> None:\n    print(value)\n", false ; "union")]
#[test_case("def f(value: Union[Any]) -> None:\n    print(value)\n", false ; "single_element_union")]
#[test_case("def f(value: int | Any) -> None:\n    print(value)\n", false ; "pep604_union")]
#[test_case("def f(value: Annotated[Any, \"meta\"]) -> None:\n    print(value)\n", false ; "annotated")]
#[test_case("def f() -> Optional[Any]:\n    return None\n", false ; "return_optional")]
#[test_case("def f(value: list[Any]) -> None:\n    print(value)\n", true ; "known_generic")]
#[test_case("def f(value: Annotated[int, Any]) -> None:\n    print(value)\n", true ; "annotated_metadata")]
#[test_case("def f(value: Foo[Any]) -> None:\n    print(value)\n", true ; "unknown_generic")]
fn imprecise_annotation_follows_ruff_optional_any(source: &str, reported_when_selected: bool) {
    let module = format!("from typing import Annotated, Any, Optional, Union\n\n\n{source}");
    let case =
        common::RuffOverlap { settings: "", select: "SMP602", ruff_code: "ANN401", files: &[("module.py", &module)] };
    assert_eq!(
        common::reported_without_and_with_ruff(&case, "`Any` accepts any value"),
        [true, reported_when_selected]
    );
}

// ANN401 skips `*args` and `**kwargs` under `allow-star-arg-any`, the `self` or `cls` of a method, and `@override`
// signatures, so SMP602 reports those while ANN401 runs.
#[test_case("", "def f(*args: Any) -> None:\n    print(args)\n", false ; "star_args")]
#[test_case("[tool.ruff.lint.flake8-annotations]\nallow-star-arg-any = true\n", "def f(*args: Any) -> None:\n    print(args)\n", true ; "star_args_allowed")]
#[test_case("[tool.ruff.lint.flake8-annotations]\nallow-star-arg-any = true\n", "def f(**kwargs: Optional[Any]) -> None:\n    print(kwargs)\n", true ; "star_kwargs_allowed")]
#[test_case("", "class C:\n    def m(self: Any) -> None:\n        pass\n", true ; "receiver")]
#[test_case("", "class C(Base):\n    @override\n    def m(self, value: Any) -> None:\n        print(value)\n", true ; "override")]
fn imprecise_annotation_follows_ann401_settings(extra: &str, source: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = [\"ANN401\"]\n{extra}")),
        ("module.py", &format!("from typing import Annotated, Any, Optional, override\n\n\n{source}")),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP602"]).1;
    assert_eq!(stdout.contains("`Any` accepts any value"), reported, "{stdout}");
}

// `object` is the one annotation both ANN401 and SMP602 accept for `*args` and `**kwargs`, so SMP602 leaves a variadic
// `object` alone whether or not ANN401 runs. A nested `object` or one on a regular parameter stays reported.
#[test_case("", "def f(*args: object) -> None:\n    print(args)\n", false ; "star_args")]
#[test_case("", "def f(**kwargs: object) -> None:\n    print(kwargs)\n", false ; "star_kwargs")]
#[test_case("[tool.ruff.lint.flake8-annotations]\nallow-star-arg-any = true\n", "def f(**kwargs: builtins.object) -> None:\n    print(kwargs)\n", false ; "star_kwargs_qualified")]
#[test_case("", "def f(*args: list[object]) -> None:\n    print(args)\n", true ; "nested_in_variadic")]
#[test_case("", "def f(value: object) -> None:\n    print(value)\n", true ; "parameter")]
fn imprecise_annotation_accepts_object_variadic(extra: &str, source: &str, reported: bool) {
    let module = format!("import builtins\n\n\n{source}");
    let case = common::RuffOverlap {
        settings: extra,
        select: "SMP602",
        ruff_code: "ANN401",
        files: &[("module.py", &module)],
    };
    assert_eq!(common::reported_without_and_with_ruff(&case, "`object` annotation"), [reported, reported]);
}
