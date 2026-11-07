use test_case::test_case;

use crate::common;

#[test_case("dataclass-frozen", "_dataclass_frozen.py", "py314" ; "dataclass_frozen")]
#[test_case("dataclass-frozen", "_dataclass_frozen_receivers.py", "py314" ; "dataclass_frozen_receivers")]
#[test_case("dataclass-frozen", "_dataclass_frozen_receiver_names.py", "py314" ; "dataclass_frozen_receiver_names")]
#[test_case("dataclass-slots", "_dataclass_slots_receiver_names.py", "py314" ; "dataclass_slots_receiver_names")]
#[test_case("dataclass-frozen", "_dataclass_frozen_runtime.py", "py314" ; "dataclass_frozen_runtime")]
#[test_case("dataclass-frozen", "_dataclass_frozen_splat.py", "py314" ; "dataclass_frozen_splat")]
#[test_case("dataclass-frozen", "dataclass_frozen_public.py", "py314" ; "dataclass_frozen_public")]
#[test_case("dataclass-slots", "_dataclass_slots.py", "py314" ; "dataclass_slots")]
#[test_case("dataclass-slots", "_dataclass_slots_quoted.py", "py314" ; "dataclass_slots_quoted_class_variable")]
#[test_case("dataclass-slots", "dataclass_slots_public.py", "py314" ; "dataclass_slots_public")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py314" ; "dataclass_kw_only")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py39" ; "dataclass_kw_only_py39")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_calls.py", "py314" ; "dataclass_kw_only_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_debug.py", "py314" ; "dataclass_kw_only_debug")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_splat_calls.py", "py314" ; "dataclass_kw_only_splat_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_public.py", "py314" ; "dataclass_kw_only_public")]
#[test_case("indistinct-error-class", "ierr_errors.py", "py314" ; "indistinct_error_class")]
#[test_case("indistinct-error-class", "ierr_app.py", "py314" ; "indistinct_error_class_catching_module")]
#[test_case("indistinct-error-class", "ierr_reexport.py", "py314" ; "indistinct_error_class_reexport")]
#[test_case("indistinct-error-class", "test_ierr_errors.py", "py314" ; "indistinct_error_class_test_module")]
fn rule(rule: &str, fixture: &str, target_version: &str) {
    common::assert_rule_snapshot(rule, fixture, target_version);
}

// Adding `slots` or `frozen` to a public dataclass of a library changes what its users can do with it, so those rules
// leave it alone; a private module and an application keep reporting.
#[test_case("dataclass-slots", "core.py", "library", 0 ; "slots_public_module_of_library")]
#[test_case("dataclass-slots", "_core.py", "library", 1 ; "slots_private_module_of_library")]
#[test_case("dataclass-slots", "core.py", "application", 1 ; "slots_public_module_of_application")]
#[test_case("dataclass-frozen", "core.py", "library", 0 ; "frozen_public_module_of_library")]
#[test_case("dataclass-frozen", "_core.py", "library", 1 ; "frozen_private_module_of_library")]
#[test_case("dataclass-frozen", "core.py", "application", 1 ; "frozen_public_module_of_application")]
fn library_public_dataclasses_keep_their_options(rule: &str, module: &str, kind: &str, reports: usize) {
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.simplint]\nproject-kind = \"{kind}\"\n")),
        (module, "from dataclasses import dataclass\n\n\n@dataclass\nclass Point:\n    x: int\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", rule]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), reports, "{stdout}");
}

// A public dataclass nested in a public class of a library is part of the API its users construct too.
#[test_case("core.py", "library", 0 ; "public_nested_in_public_module_of_library")]
#[test_case("_core.py", "library", 1 ; "nested_in_private_module_of_library")]
fn library_nested_dataclasses_keep_their_options(module: &str, kind: &str, reports: usize) {
    let source =
        "from dataclasses import dataclass\n\n\nclass Outer:\n    @dataclass\n    class Inner:\n        x: int\n";
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.simplint]\nproject-kind = \"{kind}\"\n")),
        (module, source),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-slots"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), reports, "{stdout}");
}
