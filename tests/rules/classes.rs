use test_case::test_case;

use crate::common;

const RELEASE_MODEL: &str =
    "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    name: str\n    version: str\n";

// The fixed source of `file` after `select` fixes the project made of `files`.
fn fixed_source(files: &[(&str, &str)], select: &str, file: &str) -> String {
    let root = common::project(files);
    common::fix_unsafe(root.path(), select);
    common::read(root.path(), file)
}

#[test_case("dataclass-frozen", "_dataclass_frozen.py", None ; "dataclass_frozen")]
#[test_case("dataclass-frozen", "_dataclass_frozen_receivers.py", None ; "dataclass_frozen_receivers")]
#[test_case("dataclass-frozen", "_dataclass_frozen_receiver_names.py", None ; "dataclass_frozen_receiver_names")]
#[test_case("dataclass-frozen", "_dataclass_frozen_dynamic_writes.py", None ; "dataclass_frozen_dynamic_writes")]
#[test_case("dataclass-slots", "_dataclass_slots_receiver_names.py", None ; "dataclass_slots_receiver_names")]
#[test_case("dataclass-slots", "_dataclass_slots_dynamic_writes.py", None ; "dataclass_slots_dynamic_writes")]
#[test_case("dataclass-frozen", "_dataclass_frozen_runtime.py", None ; "dataclass_frozen_runtime")]
#[test_case("dataclass-frozen", "_dataclass_frozen_splat.py", None ; "dataclass_frozen_splat")]
#[test_case("dataclass-frozen", "dataclass_frozen_public.py", None ; "dataclass_frozen_public")]
#[test_case("dataclass-frozen", "_dataclass_frozen_splat_explicit.py", None ; "dataclass_frozen_splat_explicit")]
#[test_case("dataclass-frozen", "_dataclass_frozen_comment.py", None ; "dataclass_frozen_comment")]
#[test_case("dataclass-slots", "_dataclass_slots.py", None ; "dataclass_slots")]
#[test_case("dataclass-slots", "_dataclass_slots_quoted.py", None ; "dataclass_slots_quoted_class_variable")]
#[test_case("dataclass-slots", "dataclass_frozen_public.py", None ; "dataclass_slots_public")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", None ; "dataclass_kw_only")]
#[test_case("dataclass-kw-only", "dataclass_kw_only.py", Some("py39") ; "dataclass_kw_only_py39")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_calls.py", None ; "dataclass_kw_only_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_debug.py", None ; "dataclass_kw_only_debug")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_splat_calls.py", None ; "dataclass_kw_only_splat_calls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_cls.py", None ; "dataclass_kw_only_cls")]
#[test_case("dataclass-kw-only", "dataclass_kw_only_public.py", None ; "dataclass_kw_only_public")]
#[test_case("indistinct-error-class", "ierr_errors.py", None ; "indistinct_error_class")]
#[test_case("indistinct-error-class", "ierr_app.py", None ; "indistinct_error_class_catching_module")]
#[test_case("indistinct-error-class", "ierr_reexport.py", None ; "indistinct_error_class_reexport")]
#[test_case("indistinct-error-class", "test_ierr_errors.py", None ; "indistinct_error_class_test_module")]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
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
    assert_eq!(
        fixed_source(
            &[
                ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
                ("models.py", RELEASE_MODEL,),
                ("main.py", caller),
            ],
            "dataclass-kw-only",
            "models.py"
        )
        .contains("kw_only=True"),
        fixed
    );
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

// A base imported under another name is the class it names, so `frozen` would break its subclass, as it does for the
// name itself. A base of another class keeps the fix.
#[test_case("Release", "from models import Release", false ; "imported_base")]
#[test_case("Base", "from models import Release as Base", false ; "aliased_base")]
#[test_case("Base", "from models import Other as Base", true ; "aliased_other_class")]
fn subclass_through_an_import_keeps_frozen_unset(base: &str, import: &str, fixed: bool) {
    assert_eq!(fixed_source(&[
        (
            "models.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n\nclass Other:\n    pass\n",
        ),
        ("main.py", &format!("{import}\n\n\nclass Tagged({base}):\n    pass\n")),
    ], "dataclass-frozen", "models.py").contains("frozen=True"), fixed);
}

// An instance reached through the module that defines its class, as `models.Release(...)` or an annotation
// `models.Release`, is an instance of that class, so a write to it withholds the fix.
#[test_case("release = models.Release(\"1\")\nrelease.version = \"2\"\n", false ; "dotted_constructor")]
#[test_case("def bump(release: models.Release) -> None:\n    release.version = \"2\"\n", false ; "dotted_annotation")]
#[test_case("release = models.Release(\"1\")\nprint(release)\n", true ; "dotted_constructor_without_write")]
fn module_qualified_instance_keeps_frozen_unset(caller: &str, fixed: bool) {
    assert_eq!(
        fixed_source(
            &[
                ("models.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n"),
                ("main.py", &format!("import models\n\n\n{caller}")),
            ],
            "dataclass-frozen",
            "models.py"
        )
        .contains("frozen=True"),
        fixed
    );
}

// An annotation names a class the run defines, or else an object of unknown class: `Any`, a protocol, a type alias and a
// type variable name no such class, so a write through them keeps `frozen` a candidate. A class of the run, even one
// that is no dataclass, types the write.
#[test_case("from typing import Any", "Any", false ; "any")]
#[test_case("from models import Versioned", "Versioned", false ; "protocol_of_the_run")]
#[test_case("from models import Other, Release\n\nItem = Release | Other", "Item", false ; "type_alias")]
#[test_case("from typing import TypeVar\n\nfrom models import Release\n\nModelT = TypeVar(\"ModelT\", bound=Release)", "ModelT", false ; "type_variable")]
#[test_case("from models import Release", "Release", false ; "dataclass_of_the_run")]
#[test_case("from models import Other", "Other", true ; "other_class_of_the_run")]
fn annotation_naming_no_class_of_the_run_writes_untyped(header: &str, annotation: &str, fixed: bool) {
    assert_eq!(fixed_source(&[
        (
            "models.py",
            "from dataclasses import dataclass\nfrom typing import Protocol\n\n\n@dataclass\nclass Release:\n    version: str\n\n\nclass Other:\n    version: str\n\n\nclass Versioned(Protocol):\n    version: str\n",
        ),
        ("main.py", &format!("{header}\n\n\ndef bump(item: {annotation}) -> None:\n    item.version = \"2\"\n")),
    ], "dataclass-frozen", "models.py").contains("frozen=True"), fixed);
}

// A field a class body annotates with a class of the run is an instance of that class in the methods, though no method
// assigns it, so a write through `self.config` keeps the class's options.
#[test_case("self.config.loaded = True", false ; "written_through_field")]
#[test_case("print(self.config.name)", true ; "read_through_field")]
fn annotated_field_is_an_instance_of_its_class(body: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n\n\n@dataclass\nclass App:\n    config: Config\n\n    def load(self) -> None:\n        {body}\n"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-frozen", "main.py")
            .contains("@dataclass(frozen=True)\nclass Config:"),
        fixed
    );
}

// A slotted class replaces a field's default, its class attribute, with a descriptor, so reading or assigning the default
// through the class name breaks it. A field without a default and a plain instance read keep the option.
#[test_case("print(Config.timeout + 1)\n", false ; "read_through_class")]
#[test_case("Config.timeout = 60\n", false ; "assigned_through_class")]
#[test_case("print(Config().timeout)\n", true ; "read_through_instance")]
#[test_case("print(Config.name)\n", true ; "read_of_field_without_default")]
fn class_level_use_of_a_default_keeps_slots_unset(module_level: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n    timeout: int = 30\n\n\n{module_level}"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-slots", "main.py")
            .contains("@dataclass(slots=True)\nclass Config:"),
        fixed
    );
}

#[test_case("        return cls.timeout\n", false ; "read_through_cls")]
#[test_case("        cls.timeout = 0\n", false ; "assigned_through_cls")]
#[test_case("        cls.total = 0\n", true ; "assigned_non_field_through_cls")]
fn classmethod_use_of_a_default_keeps_slots_unset(body: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n    timeout: int = 30\n\n    @classmethod\n    def reset(cls) -> int:\n{body}"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-slots", "main.py")
            .contains("@dataclass(slots=True)\nclass Config:"),
        fixed
    );
}

// A nested class has instances of its own, so the writes its methods make to `self` leave the dataclass's options alone.
#[test]
fn nested_class_writes_keep_the_dataclass_options() {
    let root = common::project(&[(
        "main.py",
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n    class Meta:\n        def mark(self) -> None:\n            self.flag = True\n",
    )]);
    common::fix_unsafe(root.path(), "dataclass-frozen,dataclass-slots");
    assert!(common::read(root.path(), "main.py").contains("@dataclass(frozen=True, slots=True)\nclass Release:"));
}

// A class keyword such as `metaclass=` makes the class's slotted copy a second class, so `slots=True` is an unsafe fix.
#[test_case("class Release(metaclass=ABCMeta):", "from abc import ABCMeta", false ; "metaclass")]
#[test_case("class Release:", "", true ; "no_keyword")]
fn class_keyword_makes_the_slots_fix_unsafe(class: &str, import: &str, fixed: bool) {
    let source = format!("{import}\nfrom dataclasses import dataclass\n\n\n@dataclass\n{class}\n    version: str\n");
    let root = common::project(&[("main.py", &source)]);
    common::check(root.path(), &["--no-cache", "--select", "dataclass-slots", "--fix", "."]);
    assert_eq!(common::read(root.path(), "main.py").contains("slots=True"), fixed);
    common::fix_unsafe(root.path(), "dataclass-slots");
    assert!(common::read(root.path(), "main.py").contains("slots=True"));
}

// `frozen=True` does not build the class again, so a class keyword such as `metaclass=` keeps the frozen fix safe.
#[test]
fn class_keyword_keeps_the_frozen_fix_safe() {
    let source = "from abc import ABCMeta\nfrom dataclasses import dataclass\n\n\n@dataclass\nclass Release(metaclass=ABCMeta):\n    version: str\n";
    let root = common::project(&[("main.py", source)]);
    common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen", "--fix", "."]);
    assert!(common::read(root.path(), "main.py").contains("@dataclass(frozen=True)\nclass Release"));
}

// A positional walrus becomes the value of its keyword, which needs its own parentheses to stay valid Python.
#[test_case("Release(name := raw.strip(), version=raw)", "Release(name=(name := raw.strip()), version=raw)" ; "bare_walrus")]
#[test_case("Release((name := raw.strip()), version=raw)", "Release(name=(name := raw.strip()), version=raw)" ; "parenthesized_walrus")]
fn walrus_argument_keeps_valid_keyword_syntax(call: &str, expected: &str) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    name: str\n    version: str\n\n\ndef build(raw: str) -> Release:\n    return {call}\n"
    );
    let root = common::project(&[("main.py", &source)]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    assert!(common::read(root.path(), "main.py").contains(expected));
}

// A call in an annotation is a call of the class, so its positional arguments are rewritten with the others, and a
// class named as the type of a parameter is not a use.
#[test]
fn positional_call_in_an_annotation_is_rewritten() {
    let source = "from dataclasses import dataclass\nfrom typing import Annotated\n\n\n@dataclass\nclass Doc:\n    text: str\n\n\ndef load(path: Annotated[str, Doc(\"where\")]) -> Doc:\n    return Doc(\"x\")\n";
    let root = common::project(&[("main.py", source)]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    let fixed = common::read(root.path(), "main.py");
    assert!(fixed.contains("Annotated[str, Doc(text=\"where\")]"), "{fixed}");
    assert!(fixed.contains("@dataclass(kw_only=True)"), "{fixed}");
}

// A class passed to a call in an annotation is a value the call may take, so the fix is withheld for it.
#[test]
fn class_passed_to_a_call_in_an_annotation_keeps_its_positional_calls() {
    let root = common::project(&[
        ("models.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Doc:\n    text: str\n"),
        (
            "main.py",
            "from typing import Annotated\n\nfrom models import Doc\n\n\ndef load(path: Annotated[str, make(Doc)]) -> None:\n    pass\n",
        ),
    ]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    assert!(!common::read(root.path(), "models.py").contains("kw_only=True"));
}

// A `__dict__` store through a name an annotation types as a class of the run withholds `frozen`, as a frozen instance
// rejects it; through a name the run does not define, it is no store on the class.
#[test_case("Counter", false ; "annotated_dict_store")]
#[test_case("Any", true ; "unknown_class_dict_store")]
fn annotated_dict_store_keeps_frozen_unset(annotation: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\nfrom typing import Any\n\n\n@dataclass\nclass Counter:\n    count: int\n\n\ndef reset(counter: {annotation}) -> None:\n    counter.__dict__ = {{\"count\": 0}}\n"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-frozen", "main.py")
            .contains("@dataclass(frozen=True)\nclass Counter:"),
        fixed
    );
}

// A nested class writes its own instances through the first parameter of its methods, which leaves the options alone.
// A write through any other name, the dataclass's instance in a closure or a parameter, still makes the fix unsafe.
#[test_case("    class Meta:\n        def mark(self) -> None:\n            self.flag = True\n", true ; "own_instance")]
#[test_case("    class Meta:\n        def mark(self, outer) -> None:\n            outer.flag = 1\n", false ; "unannotated_parameter")]
#[test_case("    def helper(self) -> None:\n        outer = self\n\n        class Marker:\n            def mark(self) -> None:\n                outer.flag = 1\n\n        Marker().mark()\n", false ; "closure")]
#[test_case("    def helper(self) -> None:\n        me = self\n\n        class Marker:\n            def mark(inner) -> None:\n                me.flag = 1\n\n        Marker().mark()\n", false ; "closure_write_in_method_of_nested_class")]
fn nested_class_writes_keep_options_only_for_their_own_instances(body: &str, fixed: bool) {
    let source =
        format!("from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n{body}");
    let root = common::project(&[("main.py", &source)]);
    common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen,dataclass-slots", "--fix", "."]);
    assert_eq!(
        common::read(root.path(), "main.py").contains("@dataclass(frozen=True, slots=True)\nclass Release:"),
        fixed
    );
}

// A module-level name bound to a class, `Base = Release`, names that class, so a subclass through it keeps `frozen`
// unset; a name bound to another class keeps the fix.
#[test_case("from models import Release\n\nBase = Release\n\n\nclass Tagged(Base):\n    pass\n", false ; "imported_class")]
#[test_case("from models import Other\n\nBase = Other\n\n\nclass Tagged(Base):\n    pass\n", true ; "other_class")]
fn subclass_through_a_name_bound_to_a_class_keeps_frozen_unset(caller: &str, fixed: bool) {
    assert_eq!(fixed_source(&[
        (
            "models.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n\nclass Other:\n    pass\n",
        ),
        ("main.py", caller),
    ], "dataclass-frozen", "models.py").contains("frozen=True"), fixed);
}

#[test]
fn subclass_through_a_name_bound_to_a_class_of_the_same_file_keeps_frozen_unset() {
    let root = common::project(&[(
        "main.py",
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n\nBase = Release\n\n\nclass Tagged(Base):\n    pass\n",
    )]);
    common::fix_unsafe(root.path(), "dataclass-frozen");
    assert!(!common::read(root.path(), "main.py").contains("frozen=True"));
}

// A slotted class replaces a field's default with a descriptor, so reading or setting the default through the class breaks
// it, however the class is named: through an import alias, a name bound to it, a subclass, or `type(...)`, `__class__`,
// `getattr` and `setattr` that simplint cannot tie to it. A read of a field that has no default and an instance's read keep
// the option.
#[test_case("from models import Config as C\n\nprint(C.timeout + 1)\n", false ; "import_alias")]
#[test_case("from models import Config as config_class\n\nprint(config_class.timeout)\n", false ; "lowercase_import_alias")]
#[test_case("from models import Config\n\nklass = Config\nprint(klass.timeout)\n", false ; "name_bound_to_class")]
#[test_case("from models import Config\n\n\nclass Sub(Config):\n    pass\n\n\nprint(Sub.timeout)\n", false ; "subclass")]
#[test_case("import models\n\nprint(type(models.Config()).timeout)\n", false ; "type_of_unresolved_object")]
#[test_case("import models\n\nprint(models.Config().__class__.timeout)\n", false ; "dunder_class_of_unresolved_object")]
#[test_case("import models\n\nprint(getattr(models.Config, \"timeout\"))\n", false ; "getattr_of_class")]
#[test_case("import models\n\nsetattr(models.Config, \"timeout\", 5)\n", false ; "setattr_of_class")]
#[test_case("from models import Config\n\nprint(Config().timeout)\n", true ; "instance_read")]
#[test_case("from models import Config\n\nprint(Config.name)\n", true ; "read_of_field_without_default")]
fn class_level_access_to_a_default_keeps_slots_unset(caller: &str, fixed: bool) {
    assert_eq!(fixed_source(&[
        (
            "models.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n    timeout: int = 30\n",
        ),
        ("main.py", caller),
    ], "dataclass-slots", "models.py").contains("slots=True"), fixed);
}

// A weak container holds the items put into it through `weakref.ref`, so an item of a dataclass withholds `slots`: a
// `WeakSet` in another module or under an import alias, a key of a weak dictionary, a value of one, and the object of a
// `WeakMethod`. An item of unknown class and a module with no weak container keep the option.
#[test_case("import weakref\n\nfrom models import Release\n\nLIVE = weakref.WeakSet()\nLIVE.add(Release(\"1\"))\n", false ; "weak_set_of_another_module")]
#[test_case("from weakref import WeakSet as Live\n\nfrom models import Release\n\nLIVE = Live()\nrelease = Release(\"1\")\nLIVE.add(release)\n", false ; "aliased_weak_set")]
#[test_case("from weakref import WeakKeyDictionary\n\nfrom models import Release\n\nNAMES = WeakKeyDictionary()\nNAMES[Release(\"1\")] = 1\n", false ; "weak_key_dictionary")]
#[test_case("from weakref import WeakValueDictionary\n\nfrom models import Release\n\nNAMES = WeakValueDictionary()\nNAMES[\"one\"] = Release(\"1\")\n", false ; "weak_value_dictionary")]
#[test_case("from weakref import WeakMethod\n\nfrom models import Release\n\nrelease = Release(\"1\")\nmethod = WeakMethod(release.label)\n", false ; "weak_method")]
#[test_case("import weakref\n\nfrom models import Release\n\nLIVE = weakref.WeakSet()\n\n\ndef keep(item) -> None:\n    LIVE.add(item)\n", true ; "item_of_unknown_class")]
#[test_case("from models import Release\n\nNAMES = {}\nNAMES[\"one\"] = Release(\"1\")\n", true ; "no_weak_container")]
fn weak_container_holding_a_dataclass_keeps_slots_unset(caller: &str, fixed: bool) {
    assert_eq!(fixed_source(&[
        (
            "models.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n    def label(self) -> str:\n        return self.version\n",
        ),
        ("main.py", caller),
    ], "dataclass-slots", "models.py").contains("slots=True"), fixed);
}

// A class in the metadata of `Annotated` is a value, which a caller may call by position, so the keyword-only fix leaves
// the class alone. A call in the metadata is rewritten, and so is a call in a string annotation, as an unquoted one is.
#[test_case("def load(path: Annotated[str, Release]) -> None:\n    print(path)\n", false ; "bare_class_in_metadata")]
#[test_case("def load(path: Annotated[str, Release(\"where\")]) -> None:\n    print(path)\n", true ; "call_in_metadata")]
#[test_case("def load(path: \"Annotated[str, Release('where')]\") -> None:\n    print(path)\n", true ; "call_in_string_annotation")]
#[test_case("def load(path: \"Annotated[str, Release]\") -> None:\n    print(path)\n", false ; "bare_class_in_string_annotation")]
#[test_case("def load(path: \"Annotated[str, make(Release)]\") -> None:\n    print(path)\n", false ; "class_passed_in_string_annotation")]
#[test_case("def load(path: Release(\"where\")) -> None:\n    print(path)\n", true ; "call_as_annotation")]
#[test_case("def load(path: Annotated[Release]) -> None:\n    print(path)\n", true ; "single_argument_annotated")]
#[test_case("def load(path: \"Annotated[str, \" \"Release('where')]\") -> None:\n    print(path)\n", false ; "call_in_concatenated_string_annotation")]
fn annotated_metadata_keeps_keyword_only_fix_unsafe(annotation: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\nfrom typing import Annotated\n\n\n@dataclass\nclass Release:\n    name: str\n\n\n{annotation}"
    );
    let root = common::project(&[("main.py", &source)]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    let fixed_source = common::read(root.path(), "main.py");
    assert_eq!(fixed_source.contains("@dataclass(kw_only=True)"), fixed, "{fixed_source}");
    if fixed && annotation.contains("where") {
        assert!(
            fixed_source.contains("Release(name='where')") || fixed_source.contains("Release(name=\"where\")"),
            "{fixed_source}"
        );
    }
}

#[test]
fn string_annotation_call_of_another_module_needs_keyword_arguments_by_hand() {
    let root = common::project(&[
        ("models.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Doc:\n    text: str\n"),
        (
            "main.py",
            "import models\n\n\ndef load(path: \"Annotated[str, models.Doc('where')]\") -> None:\n    print(path)\n",
        ),
    ]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    assert!(!common::read(root.path(), "models.py").contains("kw_only=True"));
}
