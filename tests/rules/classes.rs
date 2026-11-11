use test_case::test_case;

use crate::common;

#[test_case("dataclass-frozen", "_dataclass_frozen.py", "py314" ; "dataclass_frozen")]
#[test_case("dataclass-frozen", "_dataclass_frozen_receivers.py", "py314" ; "dataclass_frozen_receivers")]
#[test_case("dataclass-frozen", "_dataclass_frozen_receiver_names.py", "py314" ; "dataclass_frozen_receiver_names")]
#[test_case("dataclass-frozen", "_dataclass_frozen_dynamic_writes.py", "py314" ; "dataclass_frozen_dynamic_writes")]
#[test_case("dataclass-slots", "_dataclass_slots_receiver_names.py", "py314" ; "dataclass_slots_receiver_names")]
#[test_case("dataclass-slots", "_dataclass_slots_dynamic_writes.py", "py314" ; "dataclass_slots_dynamic_writes")]
#[test_case("dataclass-frozen", "_dataclass_frozen_runtime.py", "py314" ; "dataclass_frozen_runtime")]
#[test_case("dataclass-frozen", "_dataclass_frozen_splat.py", "py314" ; "dataclass_frozen_splat")]
#[test_case("dataclass-frozen", "dataclass_frozen_public.py", "py314" ; "dataclass_frozen_public")]
#[test_case("dataclass-frozen", "_dataclass_frozen_splat_explicit.py", "py314" ; "dataclass_frozen_splat_explicit")]
#[test_case("dataclass-frozen", "_dataclass_frozen_comment.py", "py314" ; "dataclass_frozen_comment")]
#[test_case("dataclass-slots", "_dataclass_slots.py", "py314" ; "dataclass_slots")]
#[test_case("dataclass-slots", "_dataclass_slots_quoted.py", "py314" ; "dataclass_slots_quoted_class_variable")]
#[test_case("dataclass-slots", "dataclass_frozen_public.py", "py314" ; "dataclass_slots_public")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py314" ; "dataclass_kw_only")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", "py39" ; "dataclass_kw_only_py39")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_calls.py", "py314" ; "dataclass_kw_only_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_debug.py", "py314" ; "dataclass_kw_only_debug")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_splat_calls.py", "py314" ; "dataclass_kw_only_splat_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_cls.py", "py314" ; "dataclass_kw_only_cls")]
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

// A public class a public signature of a library returns is one its users construct or receive, so its options stay
// as they are, though `__all__` leaves its name out.
#[test]
fn library_dataclass_reached_by_a_signature_keeps_its_options() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        (
            "core.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    x: int\n\n\n@dataclass\nclass _Hidden:\n    x: int\n\n\ndef load() -> Config:\n    return Config(1)\n\n\n__all__ = [\"load\"]\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-slots"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), 1, "{stdout}");
}

// A package offers the names another file imports from it when its `__all__` is absent or built dynamically, so a dataclass
// it re-exports and a caller imports from the package is the library's API, while a dataclass nothing imports stays
// internal.
#[test_case("" ; "without_all")]
#[test_case("__all__ = sorted([\"Partition\"])\n" ; "dynamic_all")]
fn library_dataclass_imported_from_a_package_keeps_its_options(init: &str) {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"library\"\n"),
        ("pkg/__init__.py", &format!("from ._impl import Partition\n\n{init}")),
        (
            "pkg/_impl.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Partition:\n    topic: str\n\n\n@dataclass\nclass _Hidden:\n    topic: str\n",
        ),
        ("pkg/caller.py", "from pkg import Partition\n\nPartition(\"a\")\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-slots"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), 1, "{stdout}");
    assert!(stdout.contains("_impl.py:9:"), "{stdout}");
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

const PUBLIC_METHOD: &str = r"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    def bump(self) -> None:
        self.count += 1
";
const PRIVATE_METHOD: &str = r"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    def _bump(self) -> None:
        self.count += 1
";
const NESTED_CLASS: &str = r"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    class Options:
        pass

    def _bump(self) -> None:
        self.count += 1
";
const METACLASS: &str = r"from abc import ABCMeta
from dataclasses import dataclass


@dataclass
class Counter(metaclass=ABCMeta):
    count: int

    def _bump(self) -> None:
        self.count += 1
";
const PARENTHESES: &str = r"from dataclasses import dataclass


@dataclass
class Counter():
    count: int

    def _bump(self) -> None:
        self.count += 1
";
const FACTORY: &str = r"from dataclasses import dataclass, field


@dataclass
class Bag:
    items: list[int] = field(default_factory=list)

    def _add(self) -> None:
        self.items = [*self.items, 1]
";
const OWN_INIT: &str = r"from dataclasses import dataclass


@dataclass(init=False)
class Counter:
    count: int

    def __init__(self, count: int) -> None:
        self.count = count * 2

    def _bump(self) -> None:
        self.count += 1
";
const UNTYPED_WRITE: &str = r"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    def _read(self) -> int:
        return self.count


def reset(obj) -> None:
    obj.count = 0
";
const MODULE_WRITE: &str = r"from dataclasses import dataclass


@dataclass
class Counter:
    count: int


counter = Counter(1)
counter.count += 1
";
const SETATTR: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    def _bump(self) -> None:
        setattr(self, "count", self.count + 1)
"#;
const DICT_WRITE: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    def _reset(self) -> None:
        self.__dict__ = {"count": 0}
"#;
const DOCSTRING_AND_SLOTS: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    """A counter."""

    __slots__ = ("count",)
    count: int

    def _bump(self) -> None:
        self.count += 1
"#;
const CALL_DEFAULT: &str = r"from dataclasses import dataclass


def make() -> int:
    return 1


@dataclass
class Counter:
    count: int = make()

    def _bump(self) -> None:
        self.count += 1
";
const MODULE_DICT_STORE: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    count: int


counter = Counter(1)
counter.__dict__ = {"count": 2}
"#;
const MODULE_DICT_READ: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    count: int


counter = Counter(1)
counter.__dict__.update({"count": 2})
"#;
const METHOD_SETATTR_DICT: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    count: int

    def _reset(self) -> None:
        setattr(self, "__dict__", {"count": 0})
"#;
const UNTYPED_DICT_STORE: &str = r#"from dataclasses import dataclass


@dataclass
class Counter:
    count: int


def reset(obj: object) -> None:
    obj.__dict__ = {"count": 0}
"#;
const TWO_FIELDS: &str = r"from dataclasses import dataclass


@dataclass
class Pair:
    a: int
    b: int

    def _swap(self) -> None:
        self.a, self.b = self.b, self.a
";
const SIX_FIELDS: &str = r"from dataclasses import dataclass


@dataclass
class Hold:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: int

    def _bump(self) -> None:
        self.a += 1
";
const INIT_FALSE: &str = r"from dataclasses import dataclass, field


@dataclass
class Hold:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: int = field(init=False, default=0)

    def _bump(self) -> None:
        self.a += 1
";
const KW_ONLY: &str = r"from dataclasses import KW_ONLY, dataclass


@dataclass
class Hold:
    a: int
    b: int
    c: int
    _: KW_ONLY
    d: int
    e: int

    def _bump(self) -> None:
        self.a += 1
";
const INIT_VAR: &str = r"from dataclasses import InitVar, dataclass


@dataclass
class Hold:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: InitVar[int]

    def __post_init__(self, f: int) -> None:
        self.a += f
";
const CLASS_VAR: &str = r"from dataclasses import dataclass
from typing import ClassVar


@dataclass
class Hold:
    a: int
    b: int
    c: int
    d: int
    e: int
    g: ClassVar[int] = 0

    def _bump(self) -> None:
        self.a += 1
";
const UNMUTATED: &str = "from dataclasses import dataclass\n\n\n@dataclass\nclass Plain:\n    width: int\n";
const UNMUTATED_WITH_BASE: &str = "from dataclasses import dataclass\n\n\nclass Base:\n    pass\n\n\n@dataclass\nclass Plain(Base):\n    width: int\n";

// A plain class is the only way to satisfy `frozen` for a dataclass written to after construction. Ruff's B903 and
// PLR0913 reject that plain class only in some shapes, so the dataclass is left unreported only where they do.
#[test_case("select = [\"B903\"]\n", PUBLIC_METHOD, 1 ; "b903_mutated_with_public_method")]
#[test_case("select = [\"B903\"]\n", PRIVATE_METHOD, 0 ; "b903_mutated_with_private_method")]
#[test_case("select = [\"B903\"]\n", NESTED_CLASS, 1 ; "b903_mutated_with_nested_class")]
#[test_case("select = [\"B903\"]\n", METACLASS, 1 ; "b903_class_keyword")]
#[test_case("select = [\"B903\"]\n", PARENTHESES, 1 ; "b903_empty_parentheses")]
#[test_case("select = [\"B903\"]\n", FACTORY, 1 ; "b903_default_factory")]
#[test_case("select = [\"B903\"]\n", DOCSTRING_AND_SLOTS, 0 ; "b903_docstring_and_slots")]
#[test_case("select = [\"B903\"]\n", CALL_DEFAULT, 0 ; "b903_call_default")]
#[test_case("select = [\"B903\"]\n", OWN_INIT, 1 ; "b903_own_init")]
#[test_case("select = [\"B903\"]\n", UNTYPED_WRITE, 1 ; "b903_untyped_write")]
#[test_case("select = [\"B903\"]\n", MODULE_WRITE, 0 ; "b903_module_level_instance_write")]
#[test_case("select = [\"B903\"]\n", SETATTR, 0 ; "b903_setattr_call")]
#[test_case("select = [\"PLR0913\"]\n", SIX_FIELDS, 0 ; "plr0913_mutated_six_fields")]
#[test_case("select = [\"PLR0913\"]\n", TWO_FIELDS, 1 ; "plr0913_mutated_two_fields")]
#[test_case("select = [\"PLR0913\"]\n", INIT_FALSE, 1 ; "plr0913_init_false_field_not_counted")]
#[test_case("select = [\"PLR0913\"]\n", KW_ONLY, 1 ; "plr0913_kw_only_sentinel_not_counted")]
#[test_case("select = [\"PLR0913\"]\n", INIT_VAR, 0 ; "plr0913_init_var_counted")]
#[test_case("select = [\"PLR0913\"]\n", CLASS_VAR, 1 ; "plr0913_class_var_not_counted")]
#[test_case("", PRIVATE_METHOD, 1 ; "neither_rule_selected")]
#[test_case("select = [\"B903\"]\n", UNMUTATED, 1 ; "b903_unmutated")]
#[test_case("select = [\"B903\"]\n", UNMUTATED_WITH_BASE, 1 ; "b903_unmutated_with_base")]
fn mutated_dataclass_is_left_to_ruff(ruff: &str, source: &str, reports: usize) {
    let config = format!("[tool.ruff]\npreview = true\n\n[tool.ruff.lint]\n{ruff}");
    let root = common::project(&[("pyproject.toml", &config), ("mod.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), reports, "{stdout}");
}

// A stub is not Python source that Ruff's B903 and PLR0913 check, so its dataclass keeps its report, while the same class
// in a module is left to Ruff.
#[test_case("mod.py", 0 ; "module")]
#[test_case("mod.pyi", 1 ; "stub")]
fn stub_dataclass_is_reported_under_ruff_rules(name: &str, reports: usize) {
    let config = "[tool.ruff]\npreview = true\n\n[tool.ruff.lint]\nselect = [\"B903\"]\n";
    let root = common::project(&[("pyproject.toml", config)]);
    let (_, stdout, _) = common::check_with_stdin(
        root.path(),
        &["--no-cache", "--select", "dataclass-frozen", "--stdin-filename", name, "-"],
        PRIVATE_METHOD,
    );
    assert_eq!(stdout.matches("Dataclass is missing").count(), reports, "{stdout}");
}

// A store to `__dict__` gets past nothing: a frozen instance raises on it wherever the store is, so the class gets no
// fix, while a read of `__dict__` breaks nothing and keeps the fix.
#[test_case(MODULE_DICT_STORE, false ; "module_level_store")]
#[test_case(METHOD_SETATTR_DICT, false ; "setattr_in_method")]
#[test_case(UNTYPED_DICT_STORE, true ; "store_on_unknown_object")]
#[test_case(MODULE_DICT_READ, true ; "module_level_read")]
fn dict_access_decides_the_frozen_fix(source: &str, fixable: bool) {
    let root = common::project(&[("mod.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), 1, "{stdout}");
    assert_eq!(stdout.contains("[*]"), fixable, "{stdout}");
}

// Writing `__dict__` gets past `frozen=True` in no way: a frozen instance raises on it, so the class gets no fix.
#[test]
fn dict_write_leaves_the_class_without_a_frozen_fix() {
    let root = common::project(&[("mod.py", DICT_WRITE)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), 1, "{stdout}");
    assert!(!stdout.contains("[*]"), "{stdout}");
}

// `getattr(module, "Release")` reaches a dataclass by a name no import shows, so its positional calls may come from
// anywhere, and no file gets the keyword-only fix. Without such a call the fix goes through.
#[test_case("import models\n", true ; "not_reached_by_getattr")]
#[test_case("import models\n\nRelease = getattr(models, \"Release\")(\"a\", \"1\")\n", false ; "reached_by_getattr")]
fn getattr_reached_dataclass_keeps_its_positional_calls(caller: &str, fixed: bool) {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
        (
            "models.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    name: str\n    version: str\n",
        ),
        ("main.py", caller),
    ]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    assert_eq!(common::read(root.path(), "models.py").contains("kw_only=True"), fixed);
}

// A handler that names a tuple of classes catches each class in it, in whichever module the tuple is defined, and
// `suppress` catches each class it is given, starred or not. A class no handler names is still reported.
#[test_case("", "from typing import Final\n\nfrom app.errors import AError, BError\n\nRETRY: Final = (AError, BError)\n\ntry:\n    pass\nexcept RETRY:\n    pass\n" ; "annotated_final_tuple")]
#[test_case("", "from app.errors import AError, BError\n\nRETRY: tuple[type[Exception], ...] = (AError, BError)\n\ntry:\n    pass\nexcept RETRY:\n    pass\n" ; "annotated_generic_tuple")]
#[test_case("", "from app.errors import AError, BError\n\nFIRST = (AError,)\nBOTH = (*FIRST, BError)\n\ntry:\n    pass\nexcept BOTH:\n    pass\n" ; "starred_tuple")]
#[test_case("", "import contextlib\n\nfrom app.errors import AError, BError\n\nRETRY = (AError, BError)\n\nwith contextlib.suppress(*RETRY):\n    pass\n" ; "starred_suppress")]
#[test_case("RETRY: Final = (AError, BError)\n", "from app.errors import RETRY\n\ntry:\n    pass\nexcept RETRY:\n    pass\n" ; "tuple_in_other_module")]
fn indistinct_error_class_counts_the_classes_a_tuple_or_suppress_catches(errors_extra: &str, main: &str) {
    let root = common::project(&[
        ("app/__init__.py", ""),
        (
            "app/errors.py",
            &format!(
                "from typing import Final\n\n\nclass AError(Exception):\n    pass\n\n\nclass BError(Exception):\n    pass\n\n\nclass PlainError(Exception):\n    pass\n\n\n{errors_extra}"
            ),
        ),
        ("app/main.py", main),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP704"]).1;
    assert!(stdout.contains("`PlainError`"), "{stdout}");
    assert!(!stdout.contains("`AError`") && !stdout.contains("`BError`"), "{stdout}");
}
