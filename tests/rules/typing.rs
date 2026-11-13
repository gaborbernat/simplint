use test_case::test_case;

use crate::common;

#[test_case("missing-final", "missing_final.py", None ; "missing_final")]
#[test_case("missing-final", "missing_final_typing.py", None ; "missing_final_extends_typing_import")]
#[test_case("missing-final", "missing_final_sorted.py", None ; "missing_final_keeps_isort_order")]
#[test_case("missing-final", "missing_final_order.py", None ; "missing_final_places_new_import")]
#[test_case("missing-final", "missing_final_alias.py", None ; "missing_final_uses_typing_alias")]
#[test_case("missing-final", "missing_final_module.py", None ; "missing_final_typing_module")]
#[test_case("missing-final", "missing_final_new.py", None ; "missing_final_new_import")]
#[test_case("missing-final", "missing_final_shared_docstring.py", None ; "missing_final_shared_docstring")]
#[test_case("missing-final", "missing_final_docstring_import.py", None ; "missing_final_docstring_import")]
#[test_case("missing-final", "missing_final_bare.py", None ; "missing_final_import_at_top")]
#[test_case("missing-final", "missing_final_checking.py", Some("py310") ; "missing_final_type_checking_eager")]
#[test_case("missing-final", "missing_final_checking.py", None ; "missing_final_type_checking_lazy")]
#[test_case("missing-final", "missing_final_future.py", Some("py310") ; "missing_final_type_checking_future")]
#[test_case("missing-final", "missing_final.py", Some("py37") ; "missing_final_before_python38")]
#[test_case("missing-final", "missing_final_gate.py", None ; "missing_final_version_gate")]
#[test_case("missing-final", "missing_final_branch.py", None ; "missing_final_version_branch")]
#[test_case("missing-final", "missing_final_globals.py", None ; "missing_final_globals_written")]
#[test_case("missing-final", "docs/conf.py", None ; "missing_final_sphinx_conf")]
#[test_case("imprecise-annotation", "imprecise_annotation.py", None ; "imprecise_annotation")]
#[test_case("imprecise-annotation", "imprecise_annotation_type_params.py", None ; "imprecise_annotation_type_params")]
#[test_case("imprecise-annotation", "imprecise_annotation_ruff_overlap.py", None ; "imprecise_annotation_ruff_overlap")]
#[test_case("imprecise-annotation", "imprecise_annotation_callable.py", None ; "imprecise_annotation_callable")]
#[test_case("imprecise-annotation", "imprecise_annotation_annotated.py", None ; "imprecise_annotation_annotated")]
#[test_case("callable-ellipsis", "callable_ellipsis.py", None ; "callable_ellipsis")]
#[test_case("vars-call", "vars_call.py", None ; "vars_call")]
#[test_case("vars-call", "vars_call_bound.py", None ; "vars_call_shadowed")]
#[test_case("vars-call", "vars_call_mapping.py", None ; "vars_call_mapping")]
#[test_case("mutable-module-constant", "mutable_module_constant.py", None ; "mutable_module_constant")]
#[test_case("mutable-module-constant", "docs/conf.py", None ; "mutable_module_constant_sphinx_conf")]
#[test_case(
    "mutable-module-constant", "mutable_module_constant.py", Some("py315") ; "mutable_module_constant_python315"
)]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

// A write through `globals()` under a name the code cannot read may set any name of the module, as a computed
// `setattr` does, so no name is final.
#[test_case("globals()[name] = 3" ; "computed_key")]
#[test_case("globals().update(LIMIT=3)" ; "update")]
#[test_case("globals().setdefault(name, 3)" ; "setdefault")]
#[test_case("globals().pop(name)" ; "pop")]
fn missing_final_skips_every_name_a_computed_globals_write_may_set(body: &str) {
    let root = common::project(&[("module.py", &format!("LIMIT = 1\nRATE = 2\n\n\ndef write(name):\n    {body}\n"))]);
    common::assert_report_count(root.path(), "SMP601", 0);
}

// A `globals` the module defines is not the builtin, so its calls write no module name and `LIMIT` stays reported.
#[test]
fn missing_final_ignores_a_globals_the_module_defines() {
    let root = common::project(&[(
        "module.py",
        "LIMIT = 1\n\n\ndef globals():\n    return {}\n\n\ndef write(name):\n    globals()[name] = 3\n",
    )]);
    common::assert_report_count(root.path(), "SMP601", 1);
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
    common::assert_report_count(root.path(), "SMP601", reports);
}

// The parenthesized name `(LIMIT)` takes no annotation of its own, as `(LIMIT: Final[int]) = 3` is not valid Python,
// but the other names of the file still get theirs.
#[test_case("(LIMIT) = 3\n", false ; "parenthesized_without_annotation")]
#[test_case("(LIMIT): int = 3\n", true ; "parenthesized_with_annotation")]
fn missing_final_fixes_a_parenthesized_name_only_when_annotated(source: &str, fixable: bool) {
    let root = common::project(&[("module.py", &format!("{source}OTHER = 4\n\nprint(LIMIT, OTHER)\n"))]);
    common::fix_unsafe(root.path(), "SMP601");
    let fixed = common::read(root.path(), "module.py");
    assert_eq!(
        (fixed.contains("OTHER: Final[int] = 4"), fixed.contains("(LIMIT): Final[int] = 3")),
        (true, fixable),
        "{fixed}"
    );
}

#[test_case("assert inspect.signature(callback)", 0 ; "inspected_not_called")]
#[test_case("assert callback(1)", 1 ; "called")]
#[test_case("assert callback.__call__(1)", 1 ; "explicit_dunder_call")]
fn callable_ellipsis_in_test_parameter_depends_on_use(body: &str, reports: usize) {
    let root = common::project(&[(
        "tests/test_callbacks.py",
        &format!(
            "from collections.abc import Callable\nimport inspect\n\n\ndef test_callback(callback: Callable[..., int]) \
                -> None:\n    {body}\n"
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

// Ruff's ANN401 reports an `Any` that is a whole annotation, or starts one as in `Any | None`, so SMP602 leaves it to
// Ruff whether or not the project selects ANN401.
#[test_case("def f(value: Any | None) -> None:\n    print(value)\n" ; "parameter")]
#[test_case("def f(value: Any) -> None:\n    print(value)\n" ; "whole_parameter")]
#[test_case("def f() -> Any:\n    return 1\n" ; "whole_return")]
fn imprecise_annotation_leaves_any_to_ann401(source: &str) {
    let module = format!("from typing import Any\n\n{source}");
    let root = common::project(&[("module.py", &module)]);
    let stdout = common::check(root.path(), &["--select", "SMP602"]).1;
    assert!(!stdout.contains("`Any` accepts any value"), "{stdout}");
}

// Ruff's ANN401 reports an `Any` that is, or sits in an `Optional`, a `Union`, a `|` or the first argument of
// `Annotated`, in a parameter or return annotation. SMP602 leaves those to it whether or not ANN401 is selected.
#[test_case("def f(value: Optional[Any]) -> None:\n    print(value)\n", false ; "optional")]
#[test_case("def f(value: Union[int, Any]) -> None:\n    print(value)\n", false ; "union")]
#[test_case("def f(value: Union[Any]) -> None:\n    print(value)\n", false ; "single_element_union")]
#[test_case("def f(value: int | Any) -> None:\n    print(value)\n", false ; "pep604_union")]
#[test_case("def f(value: Annotated[Any, \"meta\"]) -> None:\n    print(value)\n", false ; "annotated")]
#[test_case("def f() -> Optional[Any]:\n    return None\n", false ; "return_optional")]
#[test_case("def f(value: list[Any]) -> None:\n    print(value)\n", true ; "known_generic")]
#[test_case("def f(value: Annotated[int, Any]) -> None:\n    print(value)\n", true ; "annotated_metadata")]
#[test_case("def f(value: Foo[Any]) -> None:\n    print(value)\n", true ; "unknown_generic")]
fn imprecise_annotation_leaves_optional_any_to_ann401(source: &str, reported: bool) {
    let module = format!("from typing import Annotated, Any, Optional, Union\n\n\n{source}");
    let root = common::project(&[("module.py", &module)]);
    let stdout = common::check(root.path(), &["--select", "SMP602"]).1;
    assert_eq!(stdout.contains("`Any` accepts any value"), reported, "{stdout}");
}

// ANN401 skips `*args` and `**kwargs` under `allow-star-arg-any`, the `self` or `cls` of a method, and `@override`
// signatures, so SMP602 reports those whether or not ANN401 is selected.
#[test_case("", "def f(*args: Any) -> None:\n    print(args)\n", false ; "star_args")]
#[test_case(
    "[tool.ruff.lint.flake8-annotations]\nallow-star-arg-any = true\n",
    "def f(*args: Any) -> None:\n    print(args)\n",
    true ;
    "star_args_allowed"
)]
#[test_case(
    "[tool.ruff.lint.flake8-annotations]\nallow-star-arg-any = true\n",
    "def f(**kwargs: Optional[Any]) -> None:\n    print(kwargs)\n",
    true ;
    "star_kwargs_allowed"
)]
#[test_case("", "class C:\n    def m(self: Any) -> None:\n        pass\n", true ; "receiver")]
#[test_case(
    "", "class C(Base):\n    @override\n    def m(self, value: Any) -> None:\n        print(value)\n", true ; "override"
)]
fn imprecise_annotation_follows_ann401_settings(extra: &str, source: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff.lint]\nselect = []\n{extra}")),
        ("module.py", &format!("from typing import Annotated, Any, Optional, override\n\n\n{source}")),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP602"]).1;
    assert_eq!(stdout.contains("`Any` accepts any value"), reported, "{stdout}");
}

// `object` is the one annotation both ANN401 and SMP602 accept for `*args` and `**kwargs`, so SMP602 leaves a variadic
// `object` alone whether or not ANN401 runs. A nested `object` or one on a regular parameter stays reported.
#[test_case("", "def f(*args: object) -> None:\n    print(args)\n", false ; "star_args")]
#[test_case("", "def f(**kwargs: object) -> None:\n    print(kwargs)\n", false ; "star_kwargs")]
#[test_case(
    "[tool.ruff.lint.flake8-annotations]\nallow-star-arg-any = true\n",
    "def f(**kwargs: builtins.object) -> None:\n    print(kwargs)\n",
    false ;
    "star_kwargs_qualified"
)]
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

// Ruff's ANN401 skips the `def` that implements an `@overload` set, and reads a stub through the `if` bodies after it,
// until a `def` or a class resets it. SMP602 follows, so the `def` of the first branch is reported on its line, and the
// one after a branch with another `def` in it, or after a class, is left to ANN401.
#[test_case(
    "from typing import Any, overload\n\n\n@overload\ndef f(value: int) -> int: ...\n@overload\ndef f(value: str) -> \
        str: ...\nif __debug__:\n    def f(value: Any) -> Any:\n        return value\nelse:\n    def f(value: Any) -> Any\
        :\n        return value\n",
    9,
    2 ;
    "branch_implementation"
)]
#[test_case(
    "from typing import Any, overload\n\n\n@overload\ndef f(value: int) -> int: ...\n@overload\ndef f(value: str) -> \
        str: ...\nif __debug__:\n    def helper() -> None:\n        pass\n\n\ndef f(value: Any) -> Any:\n    return value\
        \n",
    0,
    0 ;
    "branch_def_before_implementation"
)]
#[test_case(
    "from typing import Any, overload\n\n\n@overload\ndef f(value: int) -> int: ...\n@overload\ndef f(value: str) -> \
        str: ...\nclass C:\n    pass\n\n\ndef f(value: Any) -> Any:\n    return value\n",
    0,
    0 ;
    "class_before_implementation"
)]
fn overload_implementation_follows_ruff_stub_reads(source: &str, line: usize, reported: usize) {
    let root = common::project(&[("module.py", source)]);
    let stdout = common::check(root.path(), &["--select", "SMP602"]).1;
    let reports = common::reports(&stdout, "SMP602");
    assert_eq!(
        (reports.len(), reports.iter().all(|report| report.starts_with(&format!("module.py:{line}:")))),
        (reported, true),
        "{stdout}"
    );
}

// Another module may change a constant through its import, or every name through a star import, out of this module's
// sight.
#[test_case("from app.settings import PLUGINS\n\nPLUGINS.append(\"extra\")\n", 0 ; "imported_by_name")]
#[test_case("from app import settings\n\nsettings.PLUGINS.append(\"extra\")\n", 0 ; "read_through_the_module")]
#[test_case("from app.settings import *\n", 0 ; "star_imported")]
#[test_case("import sys\n\nfrom app import settings\n\nsetattr(settings, sys.argv[1], [])\n", 0 ; "computed_write")]
#[test_case("import app.settings\n", 1 ; "module_imported_only")]
fn mutable_module_constant_skips_a_constant_other_modules_reach(main: &str, reports: usize) {
    let root = common::project(&[
        ("app/__init__.py", ""),
        ("app/settings.py", "PLUGINS = [\"core\"]\n\n\ndef plugins() -> int:\n    return len(PLUGINS)\n"),
        ("app/main.py", main),
    ]);
    common::assert_report_count(root.path(), "SMP605", reports);
}

// The fix wraps a dict in `types.MappingProxyType`, which it cannot import over a name the module already binds.
#[test]
fn mutable_module_constant_leaves_a_dict_whose_wrapper_name_is_taken() {
    let root = common::project(&[(
        "settings.py",
        "MappingProxyType = dict\nLIMITS = {\"a\": 1}\n\n\ndef limit(name: str) -> int:\n    if name == \"b\":\n        \
         return 0\n    elif name in LIMITS:\n        return LIMITS[name]\n    else:\n        return -1\n",
    )]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--target-version", "py314", "--select", "SMP605"]).1;
    assert!(stdout.contains("SMP605") && !stdout.contains("SMP605 [*]"), "{stdout}");
}

// A function may rebind the constant through `globals()`, where no statement names it.
#[test]
fn mutable_module_constant_skips_a_constant_written_through_globals() {
    let root = common::project(&[(
        "settings.py",
        "PLUGINS = [\"core\"]\n\n\ndef reset() -> None:\n    globals()[\"PLUGINS\"] = []\n",
    )]);
    common::assert_report_count(root.path(), "SMP605", 0);
}
