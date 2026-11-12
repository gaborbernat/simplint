use std::fs;
use std::path::Path;

use test_case::test_case;

use crate::common;

#[test_case("future-annotations", "future_annotations.py", None ; "future_annotations")]
#[test_case("future-annotations", "future_annotations.py", Some("py310") ; "future_annotations_py310")]
#[test_case("future-annotations", "future_annotations_checking.py", None ; "future_annotations_type_checking")]
#[test_case("future-annotations", "future_annotations_joined.py", None ; "future_annotations_joined_statement")]
#[test_case("future-annotations", "future_annotations_class_bindings.py", None ; "future_annotations_class_bindings")]
#[test_case("future-annotations", "future_annotations_class_shadow.py", None ; "future_annotations_class_shadow")]
#[test_case("future-annotations", "future_annotations_quoted_union.py", None ; "future_annotations_quoted_union")]
#[test_case("future-annotations", "future_annotations_guarded_import.py", None ; "future_annotations_guarded_import")]
#[test_case("future-annotations", "future_annotations_comments.py", None ; "future_annotations_comments")]
#[test_case(
    "future-annotations", "future_annotations_trailing_comment.py", None
    ; "future_annotations_trailing_comment"
)]
#[test_case("import-cycle", "import_cycle.py", None ; "import_cycle")]
#[test_case("import-cycle", "import_cycle_partner.py", None ; "import_cycle_partner")]
#[test_case("import-cycle", "import_cycle_ring_a.py", None ; "import_cycle_ring")]
#[test_case("import-cycle", "import_cycle_chord_a.py", None ; "import_cycle_chord")]
#[test_case("import-cycle", "import_cycle_exceptions.py", None ; "import_cycle_exceptions")]
#[test_case("import-cycle", "import_cycle_lazy.py", Some("py315") ; "import_cycle_lazy")]
#[test_case("import-cycle", "import_cycle_pkg/core.py", None ; "import_cycle_package")]
#[test_case("import-cycle", "import_cycle_dead.py", None ; "import_cycle_dead")]
#[test_case("import-cycle", "import_cycle_platform.py", None ; "import_cycle_platform")]
#[test_case("import-cycle", "import_cycle_platform_partner.py", None ; "import_cycle_platform_partner")]
#[test_case("import-cycle", "test_import_cycle_fixture.py", None ; "import_cycle_test_module")]
#[test_case("absolute-sibling-import", "package/sub/module.py", None ; "absolute_sibling_import")]
#[test_case("absolute-sibling-import", "package/__init__.py", None ; "absolute_sibling_import_init")]
#[test_case("absolute-sibling-import", "package/sub/script.py", None ; "absolute_sibling_import_script")]
#[test_case("absolute-sibling-import", "package/sub/channel_exec.py", None ; "absolute_sibling_import_channel_exec")]
#[test_case("absolute-sibling-import", "package/sub/remote_worker.py", None ; "absolute_sibling_import_remote_worker")]
#[test_case(
    "absolute-sibling-import", "package/sub/reversed_guard.py", None
    ; "absolute_sibling_import_reversed_guard"
)]
#[test_case(
    "absolute-sibling-import", "multiline_module_docstring_ok.py", None ; "absolute_sibling_import_outside_package"
)]
#[test_case("module-attribute-import", "writers/module_fix.py", None ; "module_attribute_import_fix")]
#[test_case("module-attribute-import", "writers/module_alias.py", None ; "module_attribute_import_alias")]
#[test_case(
    "module-attribute-import", "writers/module_mixed.py", None
    ; "module_attribute_import_store_leaves_other_names"
)]
#[test_case("module-attribute-import", "writers/module_undefined.py", None ; "module_attribute_import_undefined")]
#[test_case("module-attribute-import", "writers/module_value.py", None ; "module_attribute_import_value")]
#[test_case("module-attribute-import", "writers/module_type_param.py", None ; "module_attribute_import_type_param")]
#[test_case("module-attribute-import", "writers/module_parameter.py", None ; "module_attribute_import_parameter")]
#[test_case("module-attribute-import", "writers/module_shadowed.py", None ; "module_attribute_import_shadowed")]
#[test_case("module-attribute-import", "writers/module_clash.py", None ; "module_attribute_import_clash")]
#[test_case("module-attribute-import", "writers/module_rebound.py", None ; "module_attribute_import_rebound")]
#[test_case("module-attribute-import", "writers/module_dunder.py", None ; "module_attribute_import_dunder")]
#[test_case("module-attribute-import", "writers/module_third_party.py", None ; "module_attribute_import_third_party")]
#[test_case("module-attribute-import", "writers/module_patched.py", None ; "module_attribute_import_patched")]
#[test_case("module-attribute-import", "writers/module_lazy.py", Some("py315") ; "module_attribute_import_lazy")]
#[test_case("module-attribute-import", "writers/module_computed.py", None ; "module_attribute_import_computed_write")]
#[test_case(
    "module-attribute-import", "writers/module_function_import.py", None
    ; "module_attribute_import_function_import"
)]
#[test_case(
    "module-attribute-import", "writers/module_branch_imports.py", None
    ; "module_attribute_import_branch_imports"
)]
#[test_case(
    "module-attribute-import", "writers/module_bound_elsewhere.py", None
    ; "module_attribute_import_bound_elsewhere"
)]
#[test_case("module-attribute-import", "writers/module_builtin_name.py", None ; "module_attribute_import_builtin_name")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", None ; "dependency_import_guard")]
#[test_case("dependency-import-guard", "dependency_import_guard.py", Some("py310") ; "dependency_import_guard_py310")]
#[test_case(
    "dependency-import-guard", "dependency_import_guard_optional.py", None
    ; "dependency_import_guard_optional"
)]
#[test_case(
    "dependency-import-guard", "dependency_import_guard_function.py", None
    ; "dependency_import_guard_function"
)]
#[test_case("up-backport-import", "up_backport_import.py", Some("py311") ; "backport_import")]
#[test_case("up-backport-import", "up_backport_import.py", Some("py310") ; "backport_import_py310")]
#[test_case("up-backport-import", "up_backport_import_single.py", Some("py311") ; "backport_import_single")]
#[test_case("up-backport-import", "up_backport_import_inline.py", Some("py311") ; "backport_import_inline")]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import.py", Some("py315")
    ; "type_checking_lazy_import"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import.py", None
    ; "type_checking_lazy_import_py314"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_shared.py", Some("py315")
    ; "type_checking_lazy_import_shared"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_attribute.py", Some("py315")
    ; "type_checking_lazy_import_attribute"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_comment.py", Some("py315")
    ; "type_checking_lazy_import_comment"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_typeshed.py", Some("py315")
    ; "type_checking_lazy_import_typeshed"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_inline.py", Some("py315")
    ; "type_checking_lazy_import_inline"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_ignored.py", Some("py315")
    ; "type_checking_lazy_import_ignored"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_module.py", Some("py315")
    ; "type_checking_lazy_import_module"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_typeshed_module.py", Some("py315")
    ; "type_checking_lazy_import_typeshed_module"
)]
#[test_case(
    "up-type-checking-lazy-import", "up_type_checking_lazy_import_already_lazy.py", Some("py315")
    ; "type_checking_lazy_import_already_lazy"
)]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list.py", Some("py315") ; "lazy_modules_list")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list.py", None ; "lazy_modules_list_py314")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_partial.py", Some("py315") ; "lazy_modules_list_partial")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_mixed.py", Some("py315") ; "lazy_modules_list_mixed")]
#[test_case(
    "up-lazy-modules-list", "up_lazy_modules_list_nonliteral.py", Some("py315")
    ; "lazy_modules_list_nonliteral"
)]
#[test_case(
    "up-lazy-modules-list", "up_lazy_modules_list_comprehension.py", Some("py315")
    ; "lazy_modules_list_comprehension"
)]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_guarded.py", Some("py315") ; "lazy_modules_list_guarded")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_inline.py", Some("py315") ; "lazy_modules_list_inline")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_tuple.py", Some("py315") ; "lazy_modules_list_tuple")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_set.py", Some("py315") ; "lazy_modules_list_set")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_nested.py", Some("py315") ; "lazy_modules_list_nested")]
#[test_case("up-lazy-modules-list", "up_lazy_modules_list_lazy.py", Some("py315") ; "lazy_modules_list_already_lazy")]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

const PROJECT: &str = "[project]\nname = \"x\"\nversion = \"0\"\n";
const HOOK: &str = "repos:\n  - repo: https://github.com/henryiii/flake8-lazy\n    hooks:\n      - id: flake8-lazy\n";

#[test_case("from ._api import API\n", 0 ; "direct_private_sibling")]
#[test_case("from . import _api\n", 1 ; "direct_private_module_object")]
#[test_case("from .public import _helper\n", 1 ; "private_symbol_from_public_module")]
#[test_case("from .public.deep import API\n", 0 ; "deep_public_path")]
#[test_case("from package import API\n", 0 ; "public_import")]
#[test_case("from package import __version__\n", 0 ; "dunder_import")]
#[test_case("from os import _exit\n", 0 ; "public_underscore_name")]
#[test_case("from ._api import _\n", 0 ; "gettext_alias")]
#[test_case("from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from _typeshed import StrPath\n", 0 ; "typeshed_types")]
#[test_case("from ._api import helper\nVALUE = helper\n", 0 ; "public_name_from_private_module")]
#[test_case("from ._api import helper as _helper\nVALUE = _helper\n", 0 ; "private_binding_from_private_child")]
#[test_case("from .public import helper as _helper\nVALUE = _helper\n", 0 ; "private_binding_from_public_sibling")]
#[test_case("from package import helper as _helper\nVALUE = _helper\n", 0 ; "private_binding_from_project_package")]
#[test_case("from external import helper as _helper\nVALUE = _helper\n", 0 ; "third_party_private_binding")]
#[test_case("from ._api import _helper\nVALUE = _helper\n", 1 ; "direct_private_symbol_from_private_module")]
#[test_case("from ._api import _helper as helper\nVALUE = helper\n", 1 ; "aliased_direct_private_symbol")]
#[test_case("from ._api import API, _helper\nVALUE = _helper\n", 1 ; "mixed_public_private_symbols")]
#[test_case("from ._api._helpers import API\n", 1 ; "two_private_modules")]
#[test_case("from ._api._helpers import _helper\n", 1 ; "deep_private_symbol")]
#[test_case("from ._api.helpers import API\n", 1 ; "private_module_below_sibling")]
#[test_case("from ._api.helpers import _helper\n", 1 ; "private_symbol_below_sibling")]
#[test_case("from .._api import API\n", 1 ; "private_parent_module")]
#[test_case("from .._api import _helper\n", 1 ; "private_parent_symbol")]
#[test_case("from .. import _api\n", 1 ; "private_parent_member")]
#[test_case("from package._api import API, Other\n", 1 ; "absolute_private_module_names")]
#[test_case("from package._api import _helper\n", 1 ; "absolute_private_module_and_name")]
#[test_case("from package._api.helpers import API\n", 1 ; "absolute_private_parent_module")]
#[test_case("from package import _api\n", 1 ; "absolute_private_member")]
#[test_case("import package._api\n", 1 ; "absolute_private_import")]
#[test_case("import package._api as api\nprint(api.make())\n", 1 ; "aliased_private_module")]
#[test_case("def load():\n    from .._api import API\n    return API\n", 1 ; "function_import")]
#[test_case("def load():\n    from ._api import _helper\n    return _helper\n", 1 ; "function_direct_private_symbol")]
fn private_import_path(source: &str, expected: usize) {
    let root = common::project(&[("package/__init__.py", ""), ("package/module.py", source)]);
    common::assert_report_count(root.path(), "SMP310", expected);
}

#[test_case("import package.public as _helper\nVALUE = _helper\n", 0 ; "private_binding_of_project_module")]
#[test_case("import package as _helper\nVALUE = _helper\n", 0 ; "private_binding_of_project_package")]
#[test_case("import external as _helper\nVALUE = _helper\n", 0 ; "third_party_private_module_binding")]
#[test_case("import package.public as public\nVALUE = public\n", 0 ; "public_binding_of_project_module")]
fn private_import_path_module_alias(source: &str, expected: usize) {
    let root =
        common::project(&[("package/__init__.py", ""), ("package/public.py", ""), ("package/module.py", source)]);
    common::assert_report_count(root.path(), "SMP310", expected);
}

#[test_case("package/public.pyi", "package/module.py", "package/__init__.py" ; "stub_module")]
#[test_case("package/public/__init__.py", "package/module.py", "package/__init__.py" ; "package")]
#[test_case("package/public/__init__.pyi", "package/module.py", "package/__init__.py" ; "stub_package")]
#[test_case("src/package/public.py", "src/package/module.py", "src/package/__init__.py" ; "src_module")]
#[test_case("src/package/public.pyi", "src/package/module.py", "src/package/__init__.py" ; "src_stub_module")]
#[test_case("src/package/public/__init__.py", "src/package/module.py", "src/package/__init__.py" ; "src_package")]
#[test_case("src/package/public/__init__.pyi", "src/package/module.py", "src/package/__init__.py" ; "src_stub_package")]
fn private_import_path_project_module_alias(target: &str, consumer: &str, package: &str) {
    let root = common::project(&[(package, ""), (target, ""), (consumer, "import package.public as _helper\n")]);
    common::assert_report_count(root.path(), "SMP310", 0);
}

#[test_case("from ._api import _helper\nVALUE = _helper\n", 1 ; "nested_direct_child")]
#[test_case("from .._api import _helper\n", 1 ; "nested_parent_relative")]
#[test_case("from ._api._helpers import _helper\n", 1 ; "nested_deep_private_path")]
#[test_case("from package.sub._api import _helper\n", 1 ; "nested_absolute_private_path")]
#[test_case("from . import _api\n", 1 ; "nested_module_object")]
fn private_import_path_nested(source: &str, expected: usize) {
    let root = common::project(&[
        ("package/__init__.py", ""),
        ("package/_api.py", "_helper = object()\n"),
        ("package/sub/__init__.py", ""),
        ("package/sub/_api.py", "_helper = object()\n"),
        ("package/sub/consumer.py", source),
    ]);
    common::assert_report_count(root.path(), "SMP310", expected);
}

// Each hint names a fix, which the `fixed` files apply: SMP310 must accept the result, so a hint never leads to another
// report. An import the hint writes in backticks must be a line of the fixed files, so a wrong hint fails here.
fn hinted_imports(message: &str) -> Vec<&str> {
    message
        .split('`')
        .skip(1)
        .step_by(2)
        .filter(|item| item.starts_with("from ") || item.starts_with("lazy from ") || item.starts_with("import "))
        .collect()
}

#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_internal/_helpers.py", "API = 1\n"),
        ("package/consumer.py", "from ._internal._helpers import API\n\nVALUE = API\n"),
    ],
    &[
        ("package/_internal/__init__.py", "from ._helpers import API\n"),
        ("package/consumer.py", "from ._internal import API\n\nVALUE = API\n"),
    ],
    "Private import `._internal._helpers`; re-export `API` in `_internal/__init__.py` and import it with `from ._internal import API`"
    ; "re_export_from_private_package"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_internal/_helpers.py", "API = 1\nmake = 2\n"),
        ("package/consumer.py", "from ._internal._helpers import API, make\n\nVALUE = API\n"),
    ],
    &[
        ("package/_internal/__init__.py", "from ._helpers import API, make\n"),
        ("package/consumer.py", "from ._internal import API, make\n\nVALUE = API\n"),
    ],
    "Private import `._internal._helpers`; re-export `API, make` in `_internal/__init__.py` and import them with \
     `from ._internal import API, make`"
    ; "re_export_of_several_names"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_internal.py", "_helper = 1\n"),
        ("package/consumer.py", "from ._internal import _helper\n\nVALUE = _helper\n"),
    ],
    &[
        ("package/_internal.py", "helper = 1\n"),
        ("package/consumer.py", "from ._internal import helper\n\nVALUE = helper\n"),
    ],
    "Private import `_helper`; rename it to a public name in `._internal` and import it under that name"
    ; "private_name_from_private_module"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "API = 1\n"),
        ("package/module.py", "from package._api import API\n\nVALUE = API\n"),
    ],
    &[
        ("package/_api.py", "API = 1\n"),
        ("package/module.py", "from ._api import API\n\nVALUE = API\n"),
    ],
    "Private import `package._api`; inside `package` import it with `from ._api import API`; elsewhere import it from a \
     public module that re-exports it"
    ; "absolute_private_module_inside_package"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "def _h() -> None: ...\n"),
        ("package/module.py", "from package._api import _h\n\n_h()\n"),
    ],
    &[
        ("package/_api.py", "def h() -> None: ...\n"),
        ("package/module.py", "from ._api import h\n\nh()\n"),
    ],
    "Private import `package._api`; rename `_h` to public names in `package._api`; inside `package` import it from \
     `._api`, elsewhere from a public module that re-exports it"
    ; "absolute_private_name_inside_package"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api/__init__.py", ""),
        ("package/_api/helpers.py", "def _h() -> None: ...\n"),
        ("package/module.py", "from package._api.helpers import _h\n\n_h()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import h\n"),
        ("package/_api/__init__.py", "from .helpers import h\n"),
        ("package/_api/helpers.py", "def h() -> None: ...\n"),
        ("package/module.py", "from package import h\n\nh()\n"),
    ],
    "Private import `package._api.helpers`; rename `_h` to public names in `package._api.helpers`, and import it from a \
     public module that re-exports it"
    ; "absolute_private_name_below_sibling"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api/__init__.py", ""),
        ("package/_api/helpers.py", "API = 1\n"),
        ("package/module.py", "from package._api.helpers import API\n\nVALUE = API\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import API\n"),
        ("package/_api/__init__.py", "from .helpers import API\n"),
        ("package/module.py", "from package import API\n\nVALUE = API\n"),
    ],
    "Private import `package._api.helpers`; import it from a public module that re-exports it"
    ; "absolute_private_parent_module"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/_api.py", "API = 1\n"),
        ("package/sub/module.py", "from .._api import API\n\nVALUE = API\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import API\n"),
        ("package/sub/module.py", "from .. import API\n\nVALUE = API\n"),
    ],
    "Private import `.._api`; re-export `API` in the `__init__.py` of the package that holds it, and import it from there"
    ; "parent_relative_private_module"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/_api.py", "def _h() -> None: ...\n"),
        ("package/sub/module.py", "from .._api import _h\n\n_h()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import h\n"),
        ("package/_api.py", "def h() -> None: ...\n"),
        ("package/sub/module.py", "from .. import h\n\nh()\n"),
    ],
    "Private import `.._api`; rename `_h` to public names in `.._api`, re-export it in the `__init__.py` of the package \
     that holds it, and import it from there"
    ; "parent_relative_private_name"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_x.py", "def what() -> None: ...\n"),
        ("package/b.py", "from ._a._x import what\n\nwhat()\n"),
    ],
    &[
        ("package/_a/__init__.py", "from ._x import what\n"),
        ("package/b.py", "from ._a import what\n\nwhat()\n"),
    ],
    "Private import `._a._x`; re-export `what` in `_a/__init__.py` and import it with `from ._a import what`"
    ; "re_export_of_two_private_layers"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/b/__init__.py", ""),
        ("package/_a/b/_c.py", "def what() -> None: ...\n"),
        ("package/m3.py", "from ._a.b._c import what\n\nwhat()\n"),
    ],
    &[
        ("package/_a/__init__.py", "from .b import what\n"),
        ("package/_a/b/__init__.py", "from ._c import what\n"),
        ("package/m3.py", "from ._a import what\n\nwhat()\n"),
    ],
    "Private import `._a.b._c`; re-export `what` in every package along `._a.b._c`: `_a/b/__init__.py`, \
     `_a/__init__.py` and import it with `from ._a import what`"
    ; "re_export_through_every_package"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_b.py", "def _c() -> None: ...\n"),
        ("package/b.py", "from ._a._b import _c\n\n_c()\n"),
    ],
    &[
        ("package/_a/__init__.py", "from ._b import c\n"),
        ("package/_a/_b.py", "def c() -> None: ...\n"),
        ("package/b.py", "from ._a import c\n\nc()\n"),
    ],
    "Private import `._a._b`; rename `_c` to public names in `._a._b`, re-export it in `_a/__init__.py` and import it from \
     `._a`"
    ; "rename_private_name_of_two_private_layers"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_b.py", "def _c() -> None: ...\n"),
        ("package/b.py", "from ._a._b import _c as c\n\nc()\n"),
    ],
    &[
        ("package/_a/__init__.py", "from ._b import c\n"),
        ("package/_a/_b.py", "def c() -> None: ...\n"),
        ("package/b.py", "from ._a import c\n\nc()\n"),
    ],
    "Private import `._a._b`; rename `_c` to public names in `._a._b`, re-export it in `_a/__init__.py` and import it from \
     `._a`"
    ; "private_name_with_alias"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_b.py", "def _c() -> None: ...\ndef _d() -> None: ...\n"),
        ("package/b.py", "from ._a._b import _c, _d\n\n_c()\n"),
    ],
    &[
        ("package/_a/__init__.py", "from ._b import c, d\n"),
        ("package/_a/_b.py", "def c() -> None: ...\ndef d() -> None: ...\n"),
        ("package/b.py", "from ._a import c, d\n\nc()\n"),
    ],
    "Private import `._a._b`; rename `_c, _d` to public names in `._a._b`, re-export them in `_a/__init__.py` and import \
     them from `._a`"
    ; "private_names_of_several"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_x.py", "def what() -> None: ...\ndef _priv() -> None: ...\n"),
        ("package/m1.py", "from ._a._x import what, _priv\n\nwhat()\n_priv()\n"),
    ],
    &[
        ("package/_a/_x.py", "def what() -> None: ...\ndef priv() -> None: ...\n"),
        ("package/_a/__init__.py", "from ._x import what, priv\n"),
        ("package/m1.py", "from ._a import what, priv\n\nwhat()\npriv()\n"),
    ],
    "Private import `._a._x`; rename `_priv` to public names in `._a._x`, re-export them in `_a/__init__.py` and import \
     them from `._a`"
    ; "mixed_public_and_private_names"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_x.py", "def what() -> None: ...\n"),
        ("package/b.py", "from ._a import _x\n\n_x.what()\n"),
    ],
    &[
        ("package/_a/__init__.py", "from ._x import what\n"),
        ("package/b.py", "from ._a import what\n\nwhat()\n"),
    ],
    "Private import `_x`; re-export the names you use in `_a/__init__.py` and import them from `._a`"
    ; "submodule_of_private_package"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "def make() -> None: ...\n"),
        ("package/sub/__init__.py", ""),
        ("package/sub/m2.py", "from package._api import make\n\nmake()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import make\n"),
        ("package/sub/m2.py", "from package import make\n\nmake()\n"),
    ],
    "Private import `package._api`; re-export `make` in `package/__init__.py` and import it with `from package import make`"
    ; "absolute_import_from_subpackage"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "def make() -> None: ...\n"),
        ("package/sub/__init__.py", ""),
        ("package/sub/m2.py", "import package._api as api\n\napi.make()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import make\n"),
        ("package/sub/m2.py", "from package import make\n\nmake()\n"),
    ],
    "Private import `package._api`; re-export the names you use in `package/__init__.py` and import them from `package`"
    ; "module_object_from_subpackage"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_x.py", "def what() -> None: ...\n"),
        ("package/m5.py", "from ._a._x import *\n"),
    ],
    &[
        ("package/_a/__init__.py", "from ._x import *\n"),
        ("package/m5.py", "from ._a import *\n"),
    ],
    "Private import `._a._x`; re-export the names you use in `_a/__init__.py` and import them from `._a`"
    ; "star_of_private_module"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/_api.py", "def make() -> None: ...\n"),
        ("package/sub/module.py", "from .._api import *\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import *\n"),
        ("package/sub/module.py", "from .. import *\n"),
    ],
    "Private import `.._api`; re-export the names you use in the `__init__.py` of the package that holds them, and import \
     them from there"
    ; "star_of_parent_module"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "def make() -> object:\n    return object()\n"),
        ("package/module.py", "from . import _api\n\nVALUE = _api.make()\n"),
    ],
    &[
        ("package/_api.py", "def make() -> object:\n    return object()\n"),
        ("package/module.py", "from ._api import make\n\nVALUE = make()\n"),
    ],
    "Private import `_api`; import the names you use from `._api`"
    ; "module_object_of_private_sibling"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "def make() -> object:\n    return object()\n"),
        ("package/module.py", "import package._api as api\n\nVALUE = api.make()\n"),
    ],
    &[
        ("package/_api.py", "def make() -> object:\n    return object()\n"),
        ("package/module.py", "from ._api import make\n\nVALUE = make()\n"),
    ],
    "Private import `package._api`; import the names you use from `._api` inside `package`, or from a public module \
     that re-exports them elsewhere"
    ; "module_object_absolute_import"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api/__init__.py", ""),
        ("package/_api/helpers.py", "def make() -> object:\n    return object()\n"),
        ("package/module.py", "import package._api.helpers as helpers\n\nVALUE = helpers.make()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import make\n"),
        ("package/_api/__init__.py", "from .helpers import make\n"),
        ("package/module.py", "from package import make\n\nVALUE = make()\n"),
    ],
    "Private import `package._api.helpers`; import the names you use from a public module that re-exports them"
    ; "module_object_absolute_deep_import"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/_api.py", "def make() -> object:\n    return object()\n"),
        ("package/sub/module.py", "from .. import _api\n\nVALUE = _api.make()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import make\n"),
        ("package/sub/module.py", "from .. import make\n\nVALUE = make()\n"),
    ],
    "Private import `_api`; re-export the names you use in the `__init__.py` of the package that holds them, and import \
     them from there"
    ; "module_object_parent_relative"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/public.py", "_helper = 1\n"),
        ("package/sub/module.py", "from ..public import _helper\n\nVALUE = _helper\n"),
    ],
    &[
        ("package/public.py", "helper = 1\n"),
        ("package/sub/module.py", "from ..public import helper\n\nVALUE = helper\n"),
    ],
    "Private import `_helper`; rename it to a public name in `..public` and import it under that name"
    ; "private_name_from_public_module"
)]
#[test_case(
    &[
        ("package/__init__.py", ""),
        ("package/_api.py", "def _h() -> None: ...\n"),
        ("package/sub/__init__.py", ""),
        ("package/sub/m.py", "from package._api import _h\n\n_h()\n"),
    ],
    &[
        ("package/__init__.py", "from ._api import h\n"),
        ("package/_api.py", "def h() -> None: ...\n"),
        ("package/sub/m.py", "from package import h\n\nh()\n"),
    ],
    "Private import `package._api`; rename `_h` to public names in `package._api`, re-export it in `package/__init__.py` \
     and import it from `package`"
    ; "absolute_private_name_from_subpackage"
)]
fn private_import_path_hint(report: &[(&str, &str)], fixed: &[(&str, &str)], hint: &str) {
    let root = common::project(report);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP310"]).1;
    assert!(stdout.contains(hint), "{stdout}");
    let written: Vec<&str> = fixed.iter().flat_map(|(_, content)| content.lines()).collect();
    for import in hinted_imports(hint) {
        assert!(written.contains(&import), "the hint writes `{import}`, which the fix does not contain:\n{stdout}");
    }
    let root = common::project(&[report, fixed].concat());
    common::assert_report_count(root.path(), "SMP310", 0);
}

#[test]
fn lazy_private_name_hint_applies_its_import() {
    let report: &[(&str, &str)] = &[
        ("package/__init__.py", ""),
        ("package/_a/__init__.py", ""),
        ("package/_a/_x.py", "def what() -> None: ...\n"),
        ("package/b.py", "lazy from ._a._x import what as w\n\nw()\n"),
    ];
    let fixed: &[(&str, &str)] = &[
        ("package/_a/__init__.py", "from ._x import what\n"),
        ("package/b.py", "lazy from ._a import what as w\n\nw()\n"),
    ];
    let args = ["--no-cache", "--select", "SMP310", "--target-version", "py315"];
    let hint = "re-export `what` in `_a/__init__.py` and import it with `lazy from ._a import what as w`";
    let stdout = common::check(common::project(report).path(), &args).1;
    assert!(stdout.contains(hint), "{stdout}");
    let written: Vec<&str> = fixed.iter().flat_map(|(_, content)| content.lines()).collect();
    assert!(written.contains(&"lazy from ._a import what as w"), "{written:?}");
    let stdout = common::check(common::project(&[report, fixed].concat()).path(), &args).1;
    assert!(common::reports(&stdout, "SMP310").is_empty(), "{stdout}");
}

#[test]
fn private_package_with_reexport_is_clean() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/_a/__init__.py", "from ._x import what\n"),
        ("pkg/_a/_x.py", "def what() -> None: ...\n"),
        ("pkg/b.py", "from ._a import what\n\nwhat()\n"),
    ]);
    common::assert_report_count(root.path(), "SMP310", 0);
    common::assert_report_count(root.path(), "SMP311", 0);
}

#[test_case(&[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/_html_c.py", "")], 3 ; "three_modules")]
#[test_case(&[("pkg/_html_a.py", ""), ("pkg/_html_b.py", "")], 0 ; "two_modules")]
#[test_case(
    &[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/_html_c.py", ""), ("pkg/_html/__init__.py", "")],
    0
    ; "package_exists"
)]
#[test_case(&[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/_html.py", "")], 0 ; "module_without_rest")]
#[test_case(&[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/html_c.py", "")], 0 ; "public_module")]
#[test_case(&[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/_htmlx_c.py", "")], 0 ; "other_prefix")]
#[test_case(
    &[("pkg/__html_a.py", ""), ("pkg/__html_b.py", ""), ("pkg/__html_c.py", "")],
    0
    ; "leading_double_underscore"
)]
#[test_case(&[("pkg/__init__.py", ""), ("pkg/_html_a.py", ""), ("pkg/_html_b.py", "")], 0 ; "dunder_init")]
#[test_case(
    &[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/_html_c.pyi", "")],
    2
    ; "stub_counts_toward_prefix"
)]
#[test_case(
    &[("pkg/_html_a.py", ""), ("pkg/_html_b.py", ""), ("pkg/_html_c.py", ""), ("pkg/_html_c.pyi", "")],
    3
    ; "stub_beside_module_counts_once"
)]
#[test_case(
    &[("pkg/_io_a.py", ""), ("pkg/_io_b.py", ""), ("pkg/_io_c.py", ""), ("pkg/_io/data.txt", "x\n")],
    3
    ; "data_directory_is_not_a_package"
)]
fn flattened_package(files: &[(&str, &str)], expected: usize) {
    let root = common::project(files);
    common::assert_report_count(root.path(), "SMP311", expected);
}

#[test]
fn flattened_package_message_names_the_prefix_and_its_siblings() {
    let root = common::project(&[("pkg/_html_url.py", ""), ("pkg/_html_parse.py", ""), ("pkg/_html_css.py", "")]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP311"]).1;
    assert!(
        stdout.contains(
            "SMP311 `_html_css.py` shares the `_html_` prefix with 2 other modules; group them in a `_html` package \
             that re-exports their names from its `__init__.py`"
        ),
        "{stdout}"
    );
}

#[test_case("py" ; "module")]
#[test_case("pyi" ; "stub")]
fn flattened_package_moves_a_conflicting_prefix_module_into_the_package(extension: &str) {
    let module = format!("pkg/_csv.{extension}");
    let root =
        common::project(&[(module.as_str(), ""), ("pkg/_csv_a.py", ""), ("pkg/_csv_b.py", ""), ("pkg/_csv_c.py", "")]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP311"]).1;
    assert!(
        stdout.contains(&format!(
            "SMP311 `_csv_a.py` shares the `_csv_` prefix with 2 other modules; move `_csv.{extension}` into a `_csv` \
             package as its `__init__.py`, which re-exports their names"
        )),
        "{stdout}"
    );
    assert_eq!(common::reports(&stdout, "SMP311").len(), 3, "{stdout}");
}

#[test]
fn flattened_package_counts_a_package_the_run_leaves_out() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/_h/__init__.py", ""),
        ("pkg/_h_a.py", ""),
        ("pkg/_h_b.py", ""),
        ("pkg/_h_c.py", ""),
    ]);
    let files = ["--no-cache", "--select", "SMP311", "pkg/_h_a.py", "pkg/_h_b.py", "pkg/_h_c.py"];
    let stdout = common::check(root.path(), &files).1;
    assert!(common::reports(&stdout, "SMP311").is_empty(), "{stdout}");
}

#[test_case("tests/a/b/test_x.py", "from a.b._x import Name\nVALUE: Name\n", 0 ; "paired_mirrored")]
#[test_case("tests/test_x.py", "from _x import Name\nVALUE: Name\n", 0 ; "paired_root")]
#[test_case("a/b/test_x.py", "from a.b._x import Name\nVALUE: Name\n", 0 ; "paired_colocated")]
#[test_case("a/b/test_x.py", "from ._x import Name\nVALUE: Name\n", 0 ; "paired_colocated_relative")]
#[test_case("tests/a/b/test_x.py", "from ._x import Name\nVALUE: Name\n", 1 ; "mirrored_relative")]
#[test_case("tests/a/b/test_x.py", "from a.b._x import _Name\nVALUE: _Name\n", 1 ; "paired_private_name")]
#[test_case("tests/a/b/test_x.py", "from a.b._other import Name\nVALUE: Name\n", 1 ; "other_private_module")]
#[test_case("tests/a/b/test_x.py", "from ._other import Name\nVALUE: Name\n", 1 ; "other_relative_module")]
#[test_case("tests/a/b/test_x.py", "from a.c._x import Name\nVALUE: Name\n", 1 ; "other_package")]
#[test_case("tests/a/b/test_x.py", "from .._x import Name\nVALUE: Name\n", 1 ; "parent_relative")]
#[test_case("tests/a/b/test_x.py", "from a.b._x.deep import Name\nVALUE: Name\n", 1 ; "nested_private_module")]
#[test_case("tests/a/b/test_x.py", "from a.b import _x\nVALUE: _x\n", 1 ; "private_module_object")]
#[test_case("tests/a/b/test_x.py", "import a.b._x\n", 1 ; "absolute_module_object")]
#[test_case("tests/a/b/test_y.py", "from a.b._x import Name\nVALUE: Name\n", 1 ; "unpaired_test")]
#[test_case("tests/a/b/conftest.py", "from a.b._x import Name\nVALUE: Name\n", 1 ; "fixture_module")]
#[test_case("a/b/consumer.py", "from a.b._x import Name\nVALUE: Name\n", 1 ; "production_absolute")]
fn private_import_path_in_tests(file: &str, source: &str, expected: usize) {
    let root = common::project(&[("a/b/_x.py", "Name = object()\n"), ("_x.py", "Name = object()\n"), (file, source)]);
    common::assert_report_count(root.path(), "SMP310", expected);
}

#[test_case(
    "tests/_render/test_intersphinx.py", "pkg._render.intersphinx", "src/pkg/_render/intersphinx.py", 0
    ; "private_parent_with_omitted_package"
)]
#[test_case(
    "tests/_resolve/guards/test_run.py", "pkg._resolve.guards.run", "src/pkg/_resolve/guards/run.py", 0
    ; "nested_private_parent"
)]
#[test_case(
    "tests/pkg/_render/test_intersphinx.py", "pkg._render.intersphinx", "src/pkg/_render/intersphinx.py", 0
    ; "private_parent_with_package"
)]
#[test_case("tests/_html/test_url.py", "pkg._html", "src/pkg/_html.pyi", 0 ; "paired_extension_suite")]
#[test_case("tests/test__objects.py", "pkg._objects", "src/pkg/_objects.py", 0 ; "private_root_module")]
#[test_case("tests/_html/url/test_url.py", "pkg._html", "src/pkg/_html.pyi", 0 ; "nested_extension_suite")]
#[test_case("tests/pkg/_html/url/test_url.py", "pkg._html", "src/pkg/_html.pyi", 0 ; "extension_suite_with_package")]
#[test_case("tests/_html/test_url.py", "pkg._other", "src/pkg/_other.py", 1 ; "different_suite_module")]
#[test_case("tests/_other/test_url.py", "pkg._html", "src/pkg/_html.pyi", 1 ; "different_suite_path")]
#[test_case("tests/_htmlish/test_url.py", "pkg._html", "src/pkg/_html.pyi", 1 ; "suite_path_prefix")]
#[test_case("tests/_render/test_intersphinx.py", "pkg._render.other", "src/pkg/_render/other.py", 1 ; "different_stem")]
#[test_case(
    "tests/_resolve/guards/test_run.py", "pkg._resolve.other.run", "src/pkg/_resolve/other/run.py", 1
    ; "different_package"
)]
#[test_case(
    "tests/_render/test_intersphinx.py", "pkg._render.intersphinx", "src/other/_render/intersphinx.py", 1
    ; "missing_imported_module"
)]
fn private_import_path_in_mirrored_tests(file: &str, module: &str, source_file: &str, expected: usize) {
    let root = common::project(&[
        (source_file, "Name = object()\n"),
        (file, &format!("from {module} import Name\nVALUE: Name\n")),
    ]);
    common::assert_report_count(root.path(), "SMP310", expected);
}

#[test]
fn private_module_object_remains_disallowed_in_paired_suite() {
    let root = common::project(&[
        ("src/pkg/_html.pyi", "Name: object\n"),
        ("tests/_html/test_url.py", "from pkg import _html\nVALUE: _html\n"),
    ]);
    common::assert_report_count(root.path(), "SMP310", 1);
}

#[test]
fn private_module_object_import_reports_both_import_and_attribute() {
    let root = common::project(&[
        ("package/__init__.py", ""),
        ("package/_api.py", "def make() -> object:\n    return object()\n"),
        ("package/consumer.py", "import package._api as api\n\nvalue = api.make()\n"),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--unsafe-fixes", "--select", "SMP304,SMP310"]).1;
    assert!(stdout.contains("SMP310 Private import `package._api`"), "{stdout}");
    assert!(stdout.contains("SMP304 Import `make` from `package._api`"), "{stdout}");
    assert!(!stdout.contains("unsafe fix available"), "{stdout}");
}

// Ruff's PLC2701 reports a private import that a module or function uses at runtime, or never uses, and leaves the
// rest to this rule: uses only in annotations, class bodies, relative imports and a module's own package. This rule
// leaves each of Ruff's reports to it whether or not the project selects PLC2701.
#[test_case("consumer.py", "from pkg._api import make\n\nVALUE = make()\n", false ; "runtime_use")]
#[test_case("consumer.py", "from pkg._api import make\n\nVALUE = 1\n", false ; "unused_import")]
#[test_case("consumer.py", "import pkg._api as api\n\nVALUE = api.make()\n", false ; "runtime_module_alias")]
#[test_case("consumer.py", "import os._exit as exit_now\n\nexit_now(0)\n", true ; "os_exit")]
#[test_case(
    "consumer.py", "def run() -> object:\n    from pkg._api import make\n\n    return make()\n", false
    ; "function_scope"
)]
#[test_case(
    "consumer.py", "class Consumer:\n    from pkg._api import make\n\n    VALUE = make()\n", true
    ; "class_body"
)]
#[test_case(
    "consumer.py", "from pkg._api import API\n\n\ndef use(value: API) -> None:\n    return None\n", true
    ; "annotation_only_use"
)]
#[test_case("consumer.py", "import pkg._api\n\nVALUE = pkg._api.make()\n", true ; "import_without_alias")]
#[test_case("consumer.py", "from pkg._api import __version__\n\nVALUE = __version__\n", true ; "dunder_member")]
#[test_case("consumer.py", "from pkg import make\n\nVALUE = make()\n", false ; "public_import")]
#[test_case("pkg/consumer.py", "from .._api import make\n\nVALUE = make()\n", true ; "relative")]
#[test_case("pkg/module.py", "from pkg._api import make\n\nVALUE = make()\n", true ; "own_package")]
#[test_case("tests/test_consumer.py", "from pkg._api import make\n\nVALUE = make()\n", false ; "unpaired_test_module")]
#[test_case(
    "tests/test_consumer.py",
    "from pkg._api import API, make\n\n\ndef use(value: API) -> object:\n    return make()\n",
    false
    ; "unpaired_test_module_any_alias"
)]
#[test_case(
    "consumer.py", "from pkg._api import API\n\n\ndef run(value: \"API\") -> None:\n    return None\n", true
    ; "string_annotation"
)]
#[test_case(
    "consumer.py", "from typing import cast\n\nfrom pkg._api import API\n\nVALUE = cast(\"API\", 1)\n", true
    ; "string_cast"
)]
#[test_case(
    "consumer.py",
    "from typing import cast\n\nfrom pkg._api import API\n\nVALUE = cast(\"API\", 1)\nKNOWN = isinstance(VALUE, API)\n",
    false
    ; "string_cast_with_runtime_use"
)]
#[test_case(
    "consumer.py", "from pkg._api import API\n\nVALUE: \"list[API]\" = []\n", true
    ; "string_variable_annotation"
)]
#[test_case(
    "consumer.py",
    "from pkg._api import API\n\n__all__ = [\"API\"]\n\n\ndef run(value: API) -> None:\n    return None\n",
    false
    ; "listed_in_all"
)]
#[test_case(
    "consumer.py", "\"\"\"Uses API.\"\"\"\nfrom pkg._api import API\n\nVALUE = 1\n", false
    ; "docstring_mention"
)]
#[test_case(
    "consumer.py", "from pkg._api import API\n\n\nclass Consumer:\n    attr: \"API | None\" = None\n", true
    ; "string_class_annotation"
)]
#[test_case(
    "consumer.py", "from typing import TypeAlias\n\nfrom pkg._api import API\n\nAlias: TypeAlias = \"API\"\n", true
    ; "string_type_alias"
)]
#[test_case(
    "consumer.py",
    "from __future__ import annotations\n\nfrom pkg._api import API\n\n__all__ = []\n__all__.append(\"API\")\n\n\ndef \
        run(value: API) -> None:\n    return None\n",
    true
    ; "appended_to_all_typing_only"
)]
#[test_case(
    "consumer.py",
    "from pkg._api import API\n\nNAME = \"API\"\n\n\ndef run(value: object) -> None:\n    return None\n",
    false
    ; "plain_string_not_a_read"
)]
fn private_import_path_follows_ruff_import_private_name(file: &str, source: &str, reported: bool) {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/_api.py", "def make() -> object:\n    return object()\n\n\nclass API:\n    pass\n"),
        (file, source),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP310"]).1;
    assert_eq!(stdout.contains("Private import `"), reported, "{stdout}");
}

fn sibling_package(prefix: &str) -> Vec<(String, &'static str)> {
    [
        ("package/__init__.py", ""),
        ("package/sub/__init__.py", ""),
        ("package/sub/sibling.py", common::CLEAN_MODULE),
        ("package/sub/module.py", "from package.sub import sibling\n"),
    ]
    .map(|(name, content)| (format!("{prefix}{name}"), content))
    .into()
}

fn sibling_import_reported(root: &Path) -> bool {
    common::check(root, &["--select", "absolute-sibling-import"]).1.contains("SMP302")
}

// flake8-lazy keeps each module's `__lazy_modules__` in step with its absolute imports, so a project that runs it keeps
// them absolute.
#[test_case("[tool.flake8-lazy]\n", None, false ; "pyproject_table")]
#[test_case("", Some((".pre-commit-config.yaml", HOOK)), false ; "pre_commit_hook")]
#[test_case("", Some((".pre-commit-config.yml", HOOK)), false ; "pre_commit_hook_yml")]
#[test_case(
    "", Some(("prek.toml", "[[repos]]\nrepo = \"x\"\nhooks = [{ id = \"flake8-lazy\" }]\n")), false
    ; "prek_hook"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: 'flake8-lazy'  # lazy imports\n")),
    false
    ; "pre_commit_hook_trailing_comment"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: flake8-lazy\t# lazy imports\n")),
    false
    ; "pre_commit_hook_tab_comment"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      - id: flake8\n        \
        additional_dependencies: [flake8-lazy]\n")),
    true
    ; "additional_dependency_reports"
)]
#[test_case(
    "",
    Some((".pre-commit-config.yaml", "repos:\n  - repo: x\n    hooks:\n      # - id: flake8-lazy\n      - id: other\
        \n")),
    true
    ; "commented_hook_reports"
)]
#[test_case("", None, true ; "unconfigured_reports")]
fn absolute_sibling_import_follows_flake8_lazy(pyproject: &str, hook: Option<(&str, &str)>, reported: bool) {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), pyproject));
    files.extend(hook.map(|(name, text)| (name.to_owned(), text)));
    assert_eq!(sibling_import_reported(common::project(&files).path()), reported);
}

// The hook of the repository that holds the project counts, as pre-commit reads it from the repository root.
#[test]
fn absolute_sibling_import_finds_hook_above_the_project() {
    let mut files = sibling_package("monorepo/pkg/");
    files.push(("monorepo/pkg/pyproject.toml".to_owned(), PROJECT));
    files.push(("monorepo/.pre-commit-config.yaml".to_owned(), HOOK));
    files.push(("monorepo/.git/HEAD".to_owned(), "ref: refs/heads/main\n"));
    assert!(!sibling_import_reported(common::project(&files).path().join("monorepo").as_path()));
}

// The search stops at the repository root, so a hook above it does not count.
#[test]
fn absolute_sibling_import_stops_at_the_repository_root() {
    let mut files = sibling_package("outer/repo/");
    files.push(("outer/.pre-commit-config.yaml".to_owned(), HOOK));
    files.push(("outer/repo/pyproject.toml".to_owned(), PROJECT));
    files.push(("outer/repo/.git/HEAD".to_owned(), "ref: refs/heads/main\n"));
    assert!(sibling_import_reported(common::project(&files).path().join("outer/repo").as_path()));
}

// Outside a repository, the search stops at the run's project, so a hook above it, as in the home directory, does not
// count.
#[test]
fn absolute_sibling_import_outside_a_repository_stops_at_the_project() {
    let mut files = sibling_package("home/repo/");
    files.push(("home/.pre-commit-config.yaml".to_owned(), HOOK));
    files.push(("home/repo/pyproject.toml".to_owned(), PROJECT));
    assert!(sibling_import_reported(common::project(&files).path().join("home/repo").as_path()));
}

// A `[tool.flake8-lazy]` table in the settings pyproject, which a workspace member's own `pyproject.toml` does not
// hold.
#[test]
fn absolute_sibling_import_finds_table_in_settings_pyproject() {
    let mut files = sibling_package("workspace/member/");
    files.push(("workspace/member/pyproject.toml".to_owned(), PROJECT));
    files.push((
        "workspace/pyproject.toml".to_owned(),
        "[tool.simplint]\n[tool.flake8-lazy.standalone]\napply = \"list\"\n",
    ));
    assert!(!sibling_import_reported(common::project(&files).path().join("workspace").as_path()));
}

// A nested project with its own `[project]` sees the hook of the project around it.
#[test]
fn absolute_sibling_import_nested_project_sees_outer_hook() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    files.extend(sibling_package("tests/fixture/"));
    files.push(("tests/fixture/pyproject.toml".to_owned(), PROJECT));
    assert!(!sibling_import_reported(common::project(&files).path()));
}

// A nested project without a `pyproject.toml` of its own takes the metadata of the project around it.
#[test]
fn absolute_sibling_import_nested_project_without_pyproject_sees_outer_hook() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    files.extend(sibling_package("tests/proj/"));
    files.push(("tests/proj/conf.py".to_owned(), ""));
    assert!(!sibling_import_reported(common::project(&files).path()));
}

// Without a `pyproject.toml`, the hook configuration of the directory the run starts from counts.
#[test]
fn absolute_sibling_import_without_pyproject_reads_hook_at_run_root() {
    let mut files = sibling_package("");
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    assert!(!sibling_import_reported(common::project(&files).path()));
}

// `--isolated` reads no pyproject and no hook configuration, so the hook of the project does not count.
#[test]
fn isolated_run_does_not_read_hook_configurations() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    files.push(("prek.toml".to_owned(), "[[repos]]\nrepo = \"x\"\nhooks = [{ id = \"flake8-lazy\" }]\n"));
    let root = common::project(&files);
    let isolated = common::check(root.path(), &["--isolated", "--select", "absolute-sibling-import"]).1;
    assert_eq!((isolated.contains("SMP302"), sibling_import_reported(root.path())), (true, false));
}

// A hook configuration that cannot be parsed counts as no hook, with a warning from the runs whose rules read it.
#[test_case("SMP101", (0, false) ; "rule_off_does_not_read_it")]
#[test_case("absolute-sibling-import", (1, true) ; "rule_on_reads_it")]
fn unparsable_hook_configuration_counts_as_no_hook(select: &str, expected: (i32, bool)) {
    let mut files = sibling_package("");
    files.push(("prek.toml".to_owned(), "[[repos]\n"));
    let root = common::project(&files);
    let (status, _, stderr) = common::check(root.path(), &["--select", select]);
    assert_eq!((status, stderr.contains("Failed to parse")), expected);
}

// A hook configuration that cannot be read counts as no hook, with a warning from the runs whose rules read it.
#[test_case("SMP101", (0, false) ; "rule_off_does_not_read_it")]
#[test_case("absolute-sibling-import", (1, true) ; "rule_on_reads_it")]
fn unreadable_hook_configuration_counts_as_no_hook(select: &str, expected: (i32, bool)) {
    let mut files = sibling_package("");
    files.push((".pre-commit-config.yaml".to_owned(), HOOK));
    let root = common::project(&files);
    common::set_mode(&root.path().join(".pre-commit-config.yaml"), 0o000);
    let (status, _, stderr) = common::check(root.path(), &["--select", select]);
    assert_eq!((status, stderr.contains("Failed to read")), expected);
}

// A hook configuration in Latin-1 is read as far as its ids go, so its other bytes do not stop the run.
#[test]
fn latin1_hook_configuration_still_counts() {
    let mut files = sibling_package("");
    files.push(("pyproject.toml".to_owned(), PROJECT));
    let root = common::project(&files);
    fs::write(
        root.path().join(".pre-commit-config.yaml"),
        b"repos:\n  - repo: x # \xe9\xe9\n    hooks:\n      - id: flake8-lazy\n",
    )
    .expect("write hook configuration");
    assert!(!sibling_import_reported(root.path()));
}

// A function that rebinds a module variable through `global` changes it after an import would have bound it.
#[test]
fn module_attribute_import_keeps_names_a_function_rebinds_through_global() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/config.py", "_run = True\n\n\ndef set_run(run: bool) -> None:\n    global _run\n    _run = run\n"),
        ("pkg/setters.py", "from . import config\n\n\ndef check() -> bool:\n    return config._run\n"),
    ]);
    let stdout = common::check(root.path(), &["--unsafe-fixes", "--select", "SMP304"]).1;
    assert!(stdout.contains("SMP304 Import `_run`"));
    assert!(!stdout.contains("SMP304 [*]"));
}

// An attribute read off a module bound by `__import__` or a plain import is an import of that attribute, not a patch of
// the module, so the module's importer is still reported.
#[test_case("numpy_api = __import__(\"numpy_api\")" ; "dunder_import")]
#[test_case("import numpy_api" ; "plain_import")]
fn module_attribute_import_reports_a_module_read_through_an_import_of_its_name(import: &str) {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/numpy_api.py", "TABLE = {\"a\": 1}\n"),
        ("pkg/gen.py", "from . import numpy_api\n\n\ndef table() -> dict:\n    return numpy_api.TABLE\n"),
        ("pkg/verify.py", &format!("def check() -> object:\n    {import}\n    return numpy_api.full_api\n")),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304", "pkg"]).1;
    assert_eq!(
        stdout,
        common::native(
            "pkg/gen.py:5:12: SMP304 Import `TABLE` from `.numpy_api` instead of reaching it through the module\
             \nFound 1 error.\n1 unsafe fix available with `--fix --unsafe-fixes`.\n"
        )
    );
}

#[test]
fn module_attribute_import_reports_each_same_named_module_read_through_an_import() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/numpy_api.py", "TABLE = {\"a\": 1}\n"),
        ("pkg/gen.py", "from . import numpy_api\n\n\ndef table() -> dict:\n    return numpy_api.TABLE\n"),
        ("tools/__init__.py", ""),
        ("tools/numpy_api.py", "TABLE = {\"a\": 1}\n"),
        ("tools/gen.py", "from . import numpy_api\n\n\ndef table() -> dict:\n    return numpy_api.TABLE\n"),
        (
            "tools/verify.py",
            "def check() -> object:\n    numpy_api = __import__(\"numpy_api\")\n    return numpy_api.full_api\n",
        ),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    assert!(stdout.contains("pkg/gen.py:5:12: SMP304 Import `TABLE`"), "{stdout}");
    assert!(stdout.contains("tools/gen.py:5:12: SMP304 Import `TABLE`"), "{stdout}");
}

#[test]
fn module_attribute_import_fixes_after_first_report_is_suppressed() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/mod.py", "def alpha() -> int:\n    return 1\n\n\ndef beta() -> int:\n    return 2\n"),
        ("pkg/use.py", "from . import mod\n\nprint(mod.alpha())  # noqa: SMP304\nprint(mod.beta())\n"),
    ]);
    common::fix_unsafe(root.path(), "SMP304");
    assert_eq!(
        common::read(root.path(), "pkg/use.py"),
        "from .mod import alpha, beta\n\nprint(alpha())  # noqa: SMP304\nprint(beta())\n"
    );
}

// `from . import b` at import time reads `VALUE` before `a` has defined it when `b` imports `a` back, so the cycle keeps
// the reports and withholds the fix. A back-import under `TYPE_CHECKING` is not a runtime import, so no cycle forms.
#[test_case(
    "from . import a\n\n\ndef helper() -> int:\n    return a.VALUE\n", false
    ; "runtime_back_import"
)]
#[test_case(
    "from typing import TYPE_CHECKING\n\nif TYPE_CHECKING:\n    from . import a\n\n\ndef helper() -> int:\n    return a.VALUE\n",
    true
    ; "type_checking_back_import"
)]
fn module_attribute_import_withholds_the_fix_in_an_import_cycle(partner: &str, fixable: bool) {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/a.py", "from . import b\n\nVALUE = 1\n\n\ndef run() -> int:\n    return b.helper()\n"),
        ("pkg/b.py", partner),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    assert_eq!(stdout.matches("SMP304 Import").count(), 2, "{stdout}");
    assert_eq!(stdout.contains("unsafe fix"), fixable, "{stdout}");
}

// The fix rewrites the reads of the module only, so a parameter, local, loop or comprehension variable of the same name
// keeps its attribute reads.
#[test]
fn module_attribute_import_fix_leaves_shadowed_reads_alone() {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/findings.py", "class DbWriter:\n    pass\n"),
        (
            "pkg/consumer.py",
            "from . import findings\n\n\ndef write(findings: object) -> object:\n    return findings.DbWriter()\n\n\n\
             writer = findings.DbWriter()\n",
        ),
    ]);
    common::fix_unsafe(root.path(), "SMP304");
    assert_eq!(
        common::read(root.path(), "pkg/consumer.py"),
        "from .findings import DbWriter\n\n\ndef write(findings: object) -> object:\n    return findings.DbWriter()\
         \n\n\nwriter = DbWriter()\n"
    );
}

// The project's own top-level package bound under another name is read through that name, as `pd.NaT` is, so only the
// reads that bypass the package's own names are reported: those through its submodules, and an alias equal to the
// package name.
#[test]
fn module_attribute_import_leaves_aliased_top_level_package_reads_alone() {
    let root = common::project(&[
        ("pkg/__init__.py", "NAT = object()\n"),
        ("pkg/core/__init__.py", "def helper() -> int:\n    return 1\n"),
        ("pkg/core/common.py", "def helper() -> int:\n    return 1\n"),
        ("aliased_top.py", "import pkg as pd\n\n\ndef check() -> object:\n    return pd.NAT\n"),
        ("same_name.py", "import pkg as pkg\n\n\ndef check() -> object:\n    return pkg.NAT\n"),
        ("aliased_submodule.py", "import pkg.core.common as com\n\n\ndef check() -> int:\n    return com.helper()\n"),
        ("aliased_package.py", "import pkg.core as core\n\n\ndef check() -> int:\n    return core.helper()\n"),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    assert!(!stdout.contains("aliased_top.py"), "{stdout}");
    for location in [
        "same_name.py:5:12: SMP304 Import `NAT`",
        "aliased_submodule.py:5:12: SMP304 Import `helper`",
        "aliased_package.py:5:12: SMP304 Import `helper`",
    ] {
        assert!(stdout.contains(&common::native(location)), "{stdout}");
    }
}

#[test_case("" ; "ruff_unselected")]
#[test_case("\"PLC2701\"" ; "ruff_selected")]
fn module_attribute_import_reports_private_module_attributes(ruff_select: &str) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff]\nlint.preview = true\nlint.select = [{ruff_select}]\n")),
        ("pkg/__init__.py", ""),
        (
            "pkg/main.py",
            "class _Limit:\n    pass\n\n\ndef _compute_max_logs(raw: str) -> int:\n    return int(raw)\n\n\ndef run() \
                -> int:\n    return 0\n",
        ),
        (
            "pkg/commands.py",
            "from . import main\n\n\ndef limit(raw: str) -> int:\n    return main._compute_max_logs(raw)\n",
        ),
        (
            "pkg/cli.py",
            "from pkg import main\n\n\ndef limit(raw: str) -> int:\n    return main._compute_max_logs(raw)\n",
        ),
        (
            "tests/test_main.py",
            "from pkg import main\n\n\ndef test_max_logs() -> None:\n    assert main._compute_max_logs(\"3\") == 3\
                \n    assert main.run() == 0\n",
        ),
        ("tests/test_types.py", "from pkg import main\n\n\ndef check(limit: main._Limit) -> None:\n    return None\n"),
        (
            "tests/test_mixed.py",
            "from pkg import main\n\n\ndef check(limit: main._Limit) -> bool:\n    return isinstance(limit, \
                main._Limit)\n",
        ),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    for location in [
        "pkg/cli.py:5:12:",
        "pkg/commands.py:5:12:",
        "tests/test_main.py:5:12:",
        "tests/test_main.py:6:12:",
        "tests/test_mixed.py:4:18:",
        "tests/test_mixed.py:5:30:",
        "tests/test_types.py:4:18:",
    ] {
        assert!(stdout.contains(&common::native(location)), "{stdout}");
    }
}

#[test_case("" ; "ruff_unselected")]
#[test_case("\"PLC2701\"" ; "ruff_selected")]
fn module_attribute_import_reports_public_names_of_private_modules(ruff_select: &str) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff]\nlint.preview = true\nlint.select = [{ruff_select}]\n")),
        ("pkg/__init__.py", ""),
        ("pkg/_store.py", "class Store:\n    pass\n"),
        (
            "tests/test_store.py",
            "import pkg._store as store\n\n\ndef test_store() -> None:\n    assert store.Store()\n",
        ),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    assert!(stdout.contains(&common::native("tests/test_store.py:5:12:")), "{stdout}");
}

#[test]
fn module_attribute_import_reports_private_names_in_typing_contexts() {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nrequires-python = \">=3.14\"\n\n[tool.ruff]\nlint.preview = true\nlint.select = [\"PLC2701\"]\n\
             lint.flake8-type-checking.runtime-evaluated-base-classes = [\"pydantic.BaseModel\"]\n",
        ),
        ("pkg/__init__.py", ""),
        ("pkg/main.py", "class _Limit:\n    pass\n\n\ndef _f() -> int:\n    return 1\n"),
        ("pkg/sub/__init__.py", ""),
        ("pkg/sub/use.py", "from pkg import main\n\n\ndef go() -> int:\n    return main._f()\n"),
        (
            "tests/test_a.py",
            "from typing import TYPE_CHECKING\n\nfrom pkg import main\n\nif TYPE_CHECKING:\n    Alias = main._Limit\n",
        ),
        ("tests/test_b.py", "from pkg import main\n\n\ndef ident[T: main._Limit](value: T) -> T:\n    return value\n"),
        (
            "tests/test_c.py",
            "from pydantic import BaseModel\n\nfrom pkg import main\n\n\nclass Model(BaseModel):\n    limit: \
                main._Limit\n",
        ),
        ("tests/test_d.py", "from pkg import main\n\n\ndef check(limit: main._Limit) -> None:\n    return None\n"),
        ("tests/test_e.py", "from pkg import main\n\ntype Alias = main._Limit\n"),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    for location in [
        "pkg/sub/use.py:5:12:",
        "tests/test_a.py:6:13:",
        "tests/test_b.py:4:14:",
        "tests/test_c.py:7:12:",
        "tests/test_d.py:4:18:",
        "tests/test_e.py:3:14:",
    ] {
        assert!(stdout.contains(&common::native(location)), "{stdout}");
    }
}

const RUNTIME_SETTINGS: &str = "lint.flake8-type-checking.runtime-evaluated-decorators = { \"fastapi.FastAPI.get\" = \
    \"required\", \"fastapi.FastAPI.post\" = \"ambiguous\" }\n\
    lint.flake8-type-checking.runtime-evaluated-base-classes = { \"pydantic.BaseModel\" = \"ambiguous\" }\n";

#[test_case("" ; "unconfigured")]
#[test_case(RUNTIME_SETTINGS ; "configured")]
fn module_attribute_import_reports_private_names_with_runtime_evaluation_settings(settings: &str) {
    let root = common::project(&[
        (
            "pyproject.toml",
            &format!(
                "[project]\nrequires-python = \">=3.14\"\n\n[tool.ruff]\nlint.preview = true\nlint.select = \
                    [\"PLC2701\"]\n{settings}"
            ),
        ),
        ("pkg/__init__.py", ""),
        ("pkg/main.py", "class _Limit:\n    pass\n"),
        (
            "tests/test_routes.py",
            "from fastapi import FastAPI\n\nfrom pkg import main\n\napp = FastAPI()\n\n\n@app.get(\"/\")\ndef \
                route(limit: main._Limit) -> None:\n    return None\n\n\n@app.post(\"/\")\ndef create(limit: \
                main._Limit) -> None:\n    return None\n",
        ),
        (
            "tests/test_models.py",
            "from pydantic import BaseModel\n\nfrom pkg import main\n\n\nclass Model(BaseModel):\n    limit: \
                main._Limit\n",
        ),
        (
            "tests/test_unlisted.py",
            "from fastapi import FastAPI\n\nfrom pkg import main\n\napp = FastAPI()\n\n\n@app.put(\"/\")\ndef \
                replace(limit: main._Limit) -> None:\n    return None\n",
        ),
        ("tests/test_other.py", "from pkg import main\n\n\nclass Other(Exception):\n    limit: main._Limit\n"),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP304"]).1;
    for location in ["tests/test_routes.py:9:18:", "tests/test_routes.py:14:19:", "tests/test_models.py:7:12:"] {
        assert!(stdout.contains(&common::native(location)), "{stdout}");
    }
    for location in ["tests/test_unlisted.py:9:20:", "tests/test_other.py:5:12:"] {
        assert!(stdout.contains(&common::native(location)), "{stdout}");
    }
}

// The builtins define these names, so their import goes. Ruff's A004 reports an unaliased import of a builtin that
// another module takes from this one or `__all__` lists, which has no fix here, so this rule leaves it to A004. The
// other imports keep their report, and the fix removes the statement only when no other module takes the name.
#[test_case(
    "from exceptiongroup import ExceptionGroup\n", "from pkg.compat import ExceptionGroup\n", false, false
    ; "imported_by_another_module"
)]
#[test_case(
    "from exceptiongroup import ExceptionGroup\n__all__ = [\"ExceptionGroup\"]\n", "", false, false
    ; "listed_in_all"
)]
#[test_case(
    "from exceptiongroup import ExceptionGroup\n", "from pkg.other import ExceptionGroup\n", true, true
    ; "imported_from_another_module"
)]
#[test_case("from exceptiongroup import ExceptionGroup\n", "from pkg import compat\n", true, true ; "module_imported")]
#[test_case(
    "from exceptiongroup import ExceptionGroup as group\n", "from pkg.compat import ExceptionGroup\n", true, false
    ; "aliased_name"
)]
#[test_case(
    "from exceptiongroup import ExceptionGroup, BaseExceptionGroup as BEG\n", "", true, false
    ; "aliased_with_builtin"
)]
fn backport_import_follows_builtin_import_shadowing(compat: &str, consumer: &str, reported: bool, removed: bool) {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"x\"\nversion = \"0\"\nrequires-python = \">=3.11\"\n"),
        ("pkg/__init__.py", ""),
        ("pkg/compat.py", compat),
        ("pkg/other.py", "ExceptionGroup = object()\n"),
        ("pkg/run.py", consumer),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP305", "pkg"]).1;
    assert_eq!(stdout.contains(&common::native("pkg/compat.py:1:1: SMP305")), reported, "{stdout}");
    common::fix_unsafe(root.path(), "SMP305");
    assert_eq!(!common::read(root.path(), "pkg/compat.py").contains("from exceptiongroup"), removed);
}

// The builtins ignorelist names a builtin that Ruff's A004 lets an import shadow, so the import keeps this rule's
// report.
#[test]
fn backport_import_keeps_report_for_a_builtin_the_ignorelist_allows() {
    let root = common::project(&[
        (
            "pyproject.toml",
            "[project]\nname = \"x\"\nversion = \"0\"\nrequires-python = \">=3.11\"\n\n[tool.ruff.lint.flake8-builtins]\
                \nignorelist = [\"ExceptionGroup\"]\n",
        ),
        ("pkg/__init__.py", ""),
        ("pkg/compat.py", "from exceptiongroup import ExceptionGroup\n"),
        ("pkg/run.py", "from pkg.compat import ExceptionGroup\n"),
    ]);
    let stdout = common::check(root.path(), &["--select", "SMP305", "pkg"]).1;
    assert!(stdout.contains(&common::native("pkg/compat.py:1:1: SMP305")), "{stdout}");
}

// A listed import that the list keeps lazy stays eager once the list goes, so the list stays. CPython compiles the body
// and the handlers of a `try` eagerly, but the `else` of a `try` without a `finally` and the `finally` lazily.
#[test_case("import json\n\nif FLAG:\n    import json as nested\n", false ; "nested_in_if")]
#[test_case("import json\n\nif FLAG:\n    pass\nelse:\n    import json as fallback\n", false ; "nested_in_else")]
#[test_case("import json\n\nwith LOCK:\n    from json import dumps\n", false ; "nested_in_with")]
#[test_case("import json\n\nfor _ in range(1):\n    import json as loop\n", false ; "nested_in_for")]
#[test_case("import json\n\nwhile FLAG:\n    from json import dumps\n", false ; "nested_in_while")]
#[test_case("import json\n\nmatch FLAG:\n    case 1:\n        import json as matched\n", false ; "nested_in_match")]
#[test_case(
    "import json\n\ntry:\n    pass\nexcept ImportError:\n    pass\nelse:\n    import json as fallback\n", false
    ; "try_else"
)]
#[test_case("import json\n\ntry:\n    pass\nfinally:\n    import json as cleanup\n", false ; "try_finally")]
#[test_case(
    "import json\n\ntry:\n    pass\nexcept ImportError:\n    pass\nelse:\n    import json as fallback\nfinally:\n    \
        pass\n",
    true
    ; "try_else_with_finally"
)]
#[test_case(
    "import json\n\ntry:\n    if FLAG:\n        import json as guarded\nexcept ImportError:\n    pass\n", true
    ; "try_body"
)]
#[test_case("import json\n\ntry:\n    pass\nexcept ImportError:\n    import json as fallback\n", true ; "try_handler")]
#[test_case("import json\n\nfrom . import helper\n", true ; "relative_import_without_a_package")]
#[test_case("import json\n\ndef load():\n    import json as local\n    return local\n", true ; "function_body")]
fn lazy_modules_list_keeps_list_for_an_import_the_fix_leaves_eager(body: &str, fixable: bool) {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"x\"\nversion = \"0\"\nrequires-python = \">=3.15\"\n"),
        ("consumer.py", &format!("__lazy_modules__ = [\"json\"]\n\n{body}")),
    ]);
    common::fix_unsafe(root.path(), "SMP307");
    assert_eq!(common::read(root.path(), "consumer.py").contains("__lazy_modules__"), !fixable);
}

// A relative import names the module after `from`, so `from . import helper` is held by a list of `pkg`, not by one of
// `pkg.helper`, and `from .helper import VALUE` is held by `pkg.helper`.
#[test_case("pkg.helper", "import pkg.helper", "from . import helper\n", true ; "relative_name_of_listed_submodule")]
#[test_case(
    "pkg.helper", "import pkg.helper", "from .helper import VALUE\n", false
    ; "relative_module_of_listed_module"
)]
#[test_case("pkg.helper", "import pkg.helper", "from . import other\n", true ; "relative_import_of_another_module")]
#[test_case("pkg", "import pkg", "from . import helper\n", false ; "relative_name_under_listed_package")]
fn lazy_modules_list_matches_a_relative_import_by_its_module(
    listed: &str,
    import: &str,
    relative: &str,
    fixable: bool,
) {
    let root = common::project(&[
        ("pyproject.toml", "[project]\nname = \"x\"\nversion = \"0\"\nrequires-python = \">=3.15\"\n"),
        ("pkg/__init__.py", ""),
        ("pkg/helper.py", common::CLEAN_MODULE),
        ("pkg/other.py", "VALUE = 2\n"),
        (
            "pkg/consumer.py",
            &format!("__lazy_modules__ = [\"{listed}\"]\n\n{import}\n{relative}\nprint(pkg.helper.VALUE)\n"),
        ),
    ]);
    common::fix_unsafe(root.path(), "SMP307");
    assert_eq!(common::read(root.path(), "pkg/consumer.py").contains("__lazy_modules__"), !fixable);
}

// A comment in the import the fix replaces would be deleted with it, so the import keeps its report and no fix.
#[test_case("from pkg import config\n", true ; "uncommented_import")]
#[test_case("from pkg import (  # keep: settings live here\n    config,\n)\n", false ; "comment_in_import")]
#[test_case("from pkg import config  # keep: settings live here\n", true ; "trailing_comment_survives")]
fn module_attribute_import_keeps_an_import_with_a_comment(import: &str, fixable: bool) {
    let root = common::project(&[
        ("pkg/__init__.py", ""),
        ("pkg/config.py", "LIMIT = 1\n"),
        ("pkg/use.py", &format!("{import}\n\ndef limit() -> int:\n    return config.LIMIT\n")),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP304", "pkg"]).1;
    assert!(stdout.contains(&common::native("pkg/use.py:")), "{stdout}");
    common::fix_unsafe(root.path(), "SMP304");
    assert_eq!(common::read(root.path(), "pkg/use.py").contains("config.LIMIT"), !fixable);
}

// A comment anywhere in the guard, including its handler, would be deleted with the guard, so the guard keeps its
// report and no fix.
#[test_case("try:\n    import json\nexcept ImportError:\n    json = None\n", true ; "uncommented_guard")]
#[test_case("try:\n    import json  # keep\nexcept ImportError:\n    json = None\n", false ; "comment_in_body")]
#[test_case(
    "try:\n    import json\nexcept ImportError:  # pragma: no cover\n    json = None\n", false
    ; "comment_in_handler"
)]
fn dependency_import_guard_keeps_a_guard_with_a_comment(guard: &str, fixable: bool) {
    let root = common::project(&[("mod.py", guard)]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP303", "mod.py"]).1;
    assert!(stdout.contains("mod.py:1:1: SMP303"), "{stdout}");
    common::fix_unsafe(root.path(), "SMP303");
    assert_eq!(common::read(root.path(), "mod.py").contains("except ImportError"), !fixable);
}

// A platform-only module raises `ImportError` on the other platforms, and CPython guards its C accelerators with a
// fallback, so a guard around them is legitimate. Other private modules, such as `_thread`, ship with every Python.
#[test_case("asyncio.unix_events", "asyncio", 0 ; "asyncio_unix_events")]
#[test_case("multiprocessing.popen_forkserver", "multiprocessing", 0 ; "multiprocessing_popen_forkserver")]
#[test_case("_pickle", "_pickle", 0 ; "accelerator_pickle")]
#[test_case("_json", "_json", 0 ; "accelerator_json")]
#[test_case("json", "json", 1 ; "ordinary_module")]
#[test_case("_thread", "_thread", 1 ; "private_thread")]
#[test_case("_collections_abc", "_collections_abc", 1 ; "private_collections_abc")]
#[test_case("_io", "_io", 1 ; "private_io")]
#[test_case("_weakref", "_weakref", 1 ; "private_weakref")]
#[test_case("concurrent.futures._base", "concurrent", 1 ; "private_submodule_base")]
#[test_case("importlib._bootstrap", "importlib", 1 ; "private_submodule_bootstrap")]
fn dependency_import_guard_ignores_platform_and_accelerator_modules(module: &str, name: &str, reports: usize) {
    let root =
        common::project(&[("mod.py", &format!("try:\n    import {module}\nexcept ImportError:\n    {name} = None\n"))]);
    common::assert_report_count(root.path(), "SMP303", reports);
}

#[test_case("import json\n\nTEXT = json.dumps([])\n", true ; "attribute")]
#[test_case("import json\nimport json\n\nTEXT = json.dumps([])\n", false ; "bound_twice")]
#[test_case("import json\n\nTEXT = json.dumps([])\nprint(json)\n", false ; "used_as_value")]
#[test_case("import json\n\nTEXT = json.__name__\n", false ; "dunder")]
fn module_attribute_import_outside_modules(source: &str, reported: bool) {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint.imports]\nmodule-attribute-scope = [\"stdlib\"]\n"),
        ("consumer.py", source),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP304"]).1;
    assert_eq!(stdout.contains("Import `"), reported, "{stdout}");
}

// Only the modules importing the package runs need what they import installed, so a plugin module, a test, a guarded
// or deferred import and an import a version check picks are left alone.
#[test_case("dependencies = [\"requests\", \"google-auth\"]\n", &["yaml"] ; "listed")]
#[test_case("dependencies = [\"requests\", \"google-auth\", \"pyyaml\"]\n", &[] ; "declared")]
#[test_case(
    "dependencies = [\"requests\", \"google-auth\"]\noptional-dependencies.yaml = [\"PyYAML\"]\n", &[]
    ; "extra"
)]
#[test_case("dynamic = [\"dependencies\"]\n", &[] ; "dynamic")]
fn undeclared_dependency_reports_imports_no_requirement_installs(project: &str, reported: &[&str]) {
    let core = "import sys\nfrom typing import TYPE_CHECKING\n\nimport requests\nimport yaml\nfrom google.auth import \
                default\n\nfrom pkg import helpers\n\ntry:\n    import rich\nexcept ImportError:\n    import \
                colorama\n\nif TYPE_CHECKING:\n    import numpy\n\nif sys.version_info < (3, 11):\n    import \
                tomli\n\n\ndef plot() -> None:\n    import matplotlib\n";
    let root = common::project(&[
        ("pyproject.toml", format!("[project]\nname = \"pkg\"\n{project}").as_str()),
        ("pkg/__init__.py", "from pkg import core\n"),
        ("pkg/core.py", core),
        ("pkg/helpers.py", ""),
        ("pkg/pytest_plugin.py", "import pytest\n"),
        ("tests/__init__.py", ""),
        ("tests/test_core.py", "import pytest\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "undeclared-dependency"]);
    let found: Vec<&str> =
        stdout.lines().filter(|line| line.contains("SMP312")).filter_map(|line| line.split('`').nth(1)).collect();
    assert_eq!(found, reported, "{stdout}");
}
