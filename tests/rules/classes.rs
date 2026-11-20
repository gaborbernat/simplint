use test_case::test_case;

use crate::common;

const LIBRARY: &str = "[tool.simplint]\nproject-kind = \"library\"\n";

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
#[test_case("dataclass-slots", "_dataclass_slots_class_cell.py", None ; "dataclass_slots_class_cell")]
#[test_case("dataclass-slots", "_dataclass_slots_class_cell.py", Some("py313") ; "dataclass_slots_class_cell_py313")]
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
#[test_case("exception-init-args", "exception_init_args.py", None ; "exception_init_args")]
#[test_case("unbound-super", "unbound_super.py", None ; "unbound_super")]
#[test_case("unbound-super", "unbound_super.py", Some("py311") ; "unbound_super_py311")]
#[test_case("broken-decorator-stack", "broken_decorator_stack.py", None ; "broken_decorator_stack")]
#[test_case("broken-decorator-stack", "broken_decorator_stack.py", Some("py310") ; "broken_decorator_stack_py310")]
#[test_case("async-sync-dunder", "async_sync_dunder.py", None ; "async_sync_dunder")]
#[test_case("rejected-operand", "rejected_operand.py", None ; "rejected_operand")]
#[test_case("too-many-attributes", "too_many_attributes.py", None ; "too_many_attributes")]
#[test_case("missing-override", "mo_child.py", None ; "missing_override")]
#[test_case("missing-override", "mo_child.py", Some("py311") ; "missing_override_py311")]
#[test_case("missing-override", "mo_shadowed.py", None ; "missing_override_shadowed_name")]
fn rule(rule: &str, fixture: &str, target_version: Option<&str>) {
    common::assert_rule_snapshot(rule, fixture, target_version.unwrap_or("py314"));
}

// Freezing a public dataclass restricts assignment. Slots can also break callers that add attributes.
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

#[test]
fn library_dataclass_reached_by_a_signature_keeps_its_options() {
    let root = common::project(&[
        ("pyproject.toml", LIBRARY),
        (
            "core.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    x: int\n\n\n@dataclass\nclass \
                _Hidden:\n    x: int\n\n\ndef load() -> Config:\n    return Config(1)\n\n\n__all__ = [\"load\"]\n",
        ),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-slots"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), 1, "{stdout}");
}

#[test_case("" ; "without_all")]
#[test_case("__all__ = sorted([\"Partition\"])\n" ; "dynamic_all")]
fn library_dataclass_imported_from_a_package_keeps_its_options(init: &str) {
    let root = common::project(&[
        ("pyproject.toml", LIBRARY),
        ("pkg/__init__.py", &format!("from ._impl import Partition\n\n{init}")),
        (
            "pkg/_impl.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Partition:\n    topic: str\n\n\n@dataclass\
                \nclass _Hidden:\n    topic: str\n",
        ),
        ("pkg/caller.py", "from pkg import Partition\n\nPartition(\"a\")\n"),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-slots"]);
    assert_eq!((stdout.matches("Dataclass is missing").count(), stdout.contains("_impl.py:9:")), (1, true), "{stdout}");
}

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
const UNMUTATED_WITH_BASE: &str = "from dataclasses import dataclass\n\n\nclass Base:\n    pass\n\n\n@dataclass\nclass \
    Plain(Base):\n    width: int\n";

// Converting a mutable dataclass to a plain class can trigger B903 or PLR0913. The exemption depends on the class and
// constructor that conversion would produce.
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
    let root = common::project(&[
        ("pyproject.toml", &format!("[tool.ruff]\npreview = true\n\n[tool.ruff.lint]\n{ruff}")),
        ("mod.py", source),
    ]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen"]);
    assert_eq!(stdout.matches("Dataclass is missing").count(), reports, "{stdout}");
}

#[test_case("mod.py", 0 ; "module")]
#[test_case("mod.pyi", 1 ; "stub")]
fn stub_dataclass_is_reported_under_ruff_rules(name: &str, reports: usize) {
    let root = common::project(&[(
        "pyproject.toml",
        "[tool.ruff]\npreview = true\n\n[tool.ruff.lint]\nselect = \
        [\"B903\"]\n",
    )]);
    let (_, stdout, _) = common::check_with_stdin(
        root.path(),
        &["--no-cache", "--select", "dataclass-frozen", "--stdin-filename", name, "-"],
        PRIVATE_METHOD,
    );
    assert_eq!(stdout.matches("Dataclass is missing").count(), reports, "{stdout}");
}

// Freezing prevents assigning a new dictionary to `instance.__dict__`. Updating the existing dictionary bypasses
// attribute assignment.
#[test_case(MODULE_DICT_STORE, false ; "module_level_store")]
#[test_case(METHOD_SETATTR_DICT, false ; "setattr_in_method")]
#[test_case(UNTYPED_DICT_STORE, true ; "store_on_unknown_object")]
#[test_case(MODULE_DICT_READ, true ; "module_level_read")]
#[test_case(DICT_WRITE, false ; "method_dict_write")]
fn dict_access_decides_the_frozen_fix(source: &str, fixable: bool) {
    let root = common::project(&[("mod.py", source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen"]);
    assert_eq!((stdout.matches("Dataclass is missing").count(), stdout.contains("[*]")), (1, fixable), "{stdout}");
}

// With an untyped receiver, the same attribute access can target several classes. A known receiver lets the rule
// withhold the fix for that class alone.
#[test_case("dataclass-slots", "", "safe" ; "slots_without_untyped_access")]
#[test_case("dataclass-slots", "weakref.ref(Node(\"a\"))\n", "none" ; "slots_weak_reference_to_a_new_instance")]
#[test_case("dataclass-slots", "vars(Node(\"a\"))\n", "none" ; "slots_vars_of_a_new_instance")]
#[test_case("dataclass-slots", "def dump(obj):\n    return vars(obj)\n", "unsafe" ; "slots_untyped_vars")]
#[test_case("dataclass-slots", "def track(obj):\n    return weakref.ref(obj)\n", "unsafe" ; "slots_untyped_weak_reference")]
#[test_case("dataclass-slots", "dump = lambda obj: obj.__dict__\n", "unsafe" ; "slots_untyped_dict_read")]
#[test_case("dataclass-frozen", "def dump(obj):\n    return vars(obj)\n", "safe" ; "frozen_untyped_vars")]
#[test_case("dataclass-frozen", "def tag(obj):\n    obj.extra = 1\n", "unsafe" ; "frozen_untyped_stranger_write")]
#[test_case("dataclass-slots", "def tag(obj):\n    obj.extra = 1\n", "unsafe" ; "slots_untyped_stranger_write")]
#[test_case(
    "dataclass-frozen",
    "class Job:\n    def __init__(self) -> None:\n        self.extra = 0\n\n\ndef tag(obj):\n    obj.extra = 1\n",
    "safe"
    ; "frozen_untyped_write_of_a_member"
)]
#[test_case("dataclass-frozen", "class Job:\n    extra = 0\n\n\ndef tag(obj):\n    obj.extra = 1\n", "safe" ; "frozen_untyped_write_of_a_class_attribute")]
#[test_case("dataclass-frozen", "Job.extra = 0\n\n\ndef tag(obj):\n    obj.extra = 1\n", "safe" ; "frozen_untyped_write_of_a_name_set_on_a_class")]
#[test_case("dataclass-frozen", "def tag(func):\n    func.__wrapped__ = 1\n", "safe" ; "frozen_untyped_dunder_write")]
fn untyped_access_decides_the_fix(rule: &str, code: &str, fix: &str) {
    let source = format!(
        "import weakref\nfrom dataclasses import dataclass\n\n\n@dataclass\nclass Node:\n    name: str\n\n\n{code}"
    );
    let root = common::project(&[("mod.py", &source)]);
    let (_, stdout, _) = common::check(root.path(), &["--no-cache", "--select", rule]);
    let found = if stdout.contains("[*]") {
        "safe"
    } else if stdout.contains("unsafe fix") {
        "unsafe"
    } else {
        "none"
    };
    assert_eq!((stdout.matches("Dataclass is missing").count(), found), (1, fix), "{stdout}");
}

// Looking up a dataclass by name can pass its constructor to a caller the index cannot see.
#[test_case("import models\n", true ; "not_reached_by_getattr")]
#[test_case("import models\n\nRelease = getattr(models, \"Release\")(\"a\", \"1\")\n", false ; "reached_by_getattr")]
fn getattr_reached_dataclass_keeps_its_positional_calls(caller: &str, fixed: bool) {
    let source = fixed_source(
        &[
            ("pyproject.toml", "[tool.simplint]\nproject-kind = \"application\"\n"),
            (
                "models.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    name: str\n    version: str\n",
            ),
            ("main.py", caller),
        ],
        "dataclass-kw-only",
        "models.py",
    );
    assert_eq!(source.contains("kw_only=True"), fixed);
}

#[test_case(
    "",
    "from typing import Final\n\nfrom app.errors import AError, BError\n\nRETRY: Final = (AError, BError)\n\ntry:\n    \
        pass\nexcept RETRY:\n    pass\n"
    ; "annotated_final_tuple"
)]
#[test_case(
    "",
    "from app.errors import AError, BError\n\nRETRY: tuple[type[Exception], ...] = (AError, BError)\n\ntry:\n    pass\
        \nexcept RETRY:\n    pass\n"
    ; "annotated_generic_tuple"
)]
#[test_case(
    "",
    "from app.errors import AError, BError\n\nFIRST = (AError,)\nBOTH = (*FIRST, BError)\n\ntry:\n    pass\nexcept \
        BOTH:\n    pass\n"
    ; "starred_tuple"
)]
#[test_case(
    "",
    "import contextlib\n\nfrom app.errors import AError, BError\n\nRETRY = (AError, BError)\n\nwith \
        contextlib.suppress(*RETRY):\n    pass\n"
    ; "starred_suppress"
)]
#[test_case(
    "RETRY: Final = (AError, BError)\n", "from app.errors import RETRY\n\ntry:\n    pass\nexcept RETRY:\n    pass\n"
    ; "tuple_in_other_module"
)]
fn indistinct_error_class_counts_the_classes_a_tuple_or_suppress_catches(errors_extra: &str, main: &str) {
    let root = common::project(&[
        ("app/__init__.py", ""),
        (
            "app/errors.py",
            &format!(
                "from typing import Final\n\n\nclass AError(Exception):\n    pass\n\n\nclass BError(Exception):\n    \
                    pass\n\n\nclass PlainError(Exception):\n    pass\n\n\n{errors_extra}"
            ),
        ),
        ("app/main.py", main),
    ]);
    let stdout = common::check(root.path(), &["--no-cache", "--select", "SMP704"]).1;
    assert_eq!(
        (stdout.contains("`PlainError`"), !stdout.contains("`AError`") && !stdout.contains("`BError`")),
        (true, true),
        "{stdout}"
    );
}

// A class referenced through an alias still has the same subclasses.
#[test_case("from models import Release\n\n\nclass Tagged(Release):\n    pass\n", false ; "imported_base")]
#[test_case("from models import Release as Base\n\n\nclass Tagged(Base):\n    pass\n", false ; "aliased_base")]
#[test_case("from models import Other as Base\n\n\nclass Tagged(Base):\n    pass\n", true ; "aliased_other_class")]
#[test_case(
    "from models import Release\n\nBase = Release\n\n\nclass Tagged(Base):\n    pass\n", false
    ; "imported_class"
)]
#[test_case("from models import Other\n\nBase = Other\n\n\nclass Tagged(Base):\n    pass\n", true ; "other_class")]
fn subclass_of_a_class_of_the_run_keeps_frozen_unset(caller: &str, fixed: bool) {
    let source = fixed_source(
        &[
            (
                "models.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n\nclass Other:\
                    \n    pass\n",
            ),
            ("main.py", caller),
        ],
        "dataclass-frozen",
        "models.py",
    );
    assert_eq!(source.contains("frozen=True"), fixed);
}

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

// The index needs a concrete project class to attribute a write to one type. A protocol or type variable can describe
// several types.
#[test_case("from typing import Any", "Any", false ; "any")]
#[test_case("from models import Versioned", "Versioned", false ; "protocol_of_the_run")]
#[test_case("from models import Other, Release\n\nItem = Release | Other", "Item", false ; "type_alias")]
#[test_case(
    "from typing import TypeVar\n\nfrom models import Release\n\nModelT = TypeVar(\"ModelT\", bound=Release)",
    "ModelT",
    false
    ; "type_variable"
)]
#[test_case("from models import Release", "Release", false ; "dataclass_of_the_run")]
#[test_case("from models import Other", "Other", true ; "other_class_of_the_run")]
fn annotation_naming_no_class_of_the_run_writes_untyped(header: &str, annotation: &str, fixed: bool) {
    let source = fixed_source(
        &[
            (
                "models.py",
                "from dataclasses import dataclass\nfrom typing import Protocol\n\n\n@dataclass\nclass Release:\n    \
                    version: str\n\n\nclass Other:\n    version: str\n\n\nclass Versioned(Protocol):\n    version: str\
                    \n",
            ),
            ("main.py", &format!("{header}\n\n\ndef bump(item: {annotation}) -> None:\n    item.version = \"2\"\n")),
        ],
        "dataclass-frozen",
        "models.py",
    );
    assert_eq!(source.contains("frozen=True"), fixed);
}

#[test_case("self.config.loaded = True", false ; "written_through_field")]
#[test_case("print(self.config.name)", true ; "read_through_field")]
fn annotated_field_is_an_instance_of_its_class(body: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n\n\n@dataclass\nclass App:\
            \n    config: Config\n\n    def load(self) -> None:\n        {body}\n"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-frozen", "main.py")
            .contains("@dataclass(frozen=True)\nclass Config:"),
        fixed
    );
}

// Slots replace a field's class-level default with a descriptor. Code accessing the field on the class can observe that
// change.
#[test_case("print(Config.timeout + 1)\n", false ; "read_through_class")]
#[test_case("Config.timeout = 60\n", false ; "assigned_through_class")]
#[test_case("print(Config().timeout)\n", true ; "read_through_instance")]
#[test_case("print(Config.name)\n", true ; "read_of_field_without_default")]
fn class_level_use_of_a_default_keeps_slots_unset(module_level: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n    timeout: int = 30\n\n\
            \n{module_level}"
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
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n    timeout: int = 30\n\n    \
            @classmethod\n    def reset(cls) -> int:\n{body}"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-slots", "main.py")
            .contains("@dataclass(slots=True)\nclass Config:"),
        fixed
    );
}

#[test]
fn nested_class_writes_keep_the_dataclass_options() {
    let root = common::project(&[(
        "main.py",
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n    class Meta:\
            \n        def mark(self) -> None:\n            self.flag = True\n",
    )]);
    common::fix_unsafe(root.path(), "dataclass-frozen,dataclass-slots");
    assert!(common::read(root.path(), "main.py").contains("@dataclass(frozen=True, slots=True)\nclass Release:"));
}

// Adding slots creates a replacement class, which can run metaclass code a second time.
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

// Freezing the class in place avoids running metaclass code a second time.
#[test]
fn class_keyword_keeps_the_frozen_fix_safe() {
    let source = "from abc import ABCMeta\nfrom dataclasses import dataclass\n\n\n@dataclass\nclass \
        Release(metaclass=ABCMeta):\n    version: str\n";
    let root = common::project(&[("main.py", source)]);
    common::check(root.path(), &["--no-cache", "--select", "dataclass-frozen", "--fix", "."]);
    assert!(common::read(root.path(), "main.py").contains("@dataclass(frozen=True)\nclass Release"));
}

// Python requires parentheses around an assignment expression used as a keyword argument value.
#[test_case(
    "Release(name := raw.strip(), version=raw)", "Release(name=(name := raw.strip()), version=raw)"
    ; "bare_walrus"
)]
#[test_case(
    "Release((name := raw.strip()), version=raw)", "Release(name=(name := raw.strip()), version=raw)"
    ; "parenthesized_walrus"
)]
fn walrus_argument_keeps_valid_keyword_syntax(call: &str, expected: &str) {
    let source = format!(
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    name: str\n    version: str\n\n\ndef \
            build(raw: str) -> Release:\n    return {call}\n"
    );
    let root = common::project(&[("main.py", &source)]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    assert!(common::read(root.path(), "main.py").contains(expected));
}

#[test]
fn positional_call_in_an_annotation_is_rewritten() {
    let source = "from dataclasses import dataclass\nfrom typing import Annotated\n\n\n@dataclass\nclass Doc:\n    \
        text: str\n\n\ndef load(path: Annotated[str, Doc(\"where\")]) -> Doc:\n    return Doc(\"x\")\n";
    let root = common::project(&[("main.py", source)]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    let fixed = common::read(root.path(), "main.py");
    assert_eq!(
        (fixed.contains("Annotated[str, Doc(text=\"where\")]"), fixed.contains("@dataclass(kw_only=True)")),
        (true, true),
        "{fixed}"
    );
}

// Passing the class as an argument can hide positional calls inside the function that receives it.
#[test]
fn class_passed_to_a_call_in_an_annotation_keeps_its_positional_calls() {
    let root = common::project(&[
        ("models.py", "from dataclasses import dataclass\n\n\n@dataclass\nclass Doc:\n    text: str\n"),
        (
            "main.py",
            "from typing import Annotated\n\nfrom models import Doc\n\n\ndef load(path: Annotated[str, make(Doc)]) -> \
                None:\n    pass\n",
        ),
    ]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    assert!(!common::read(root.path(), "models.py").contains("kw_only=True"));
}

// Assigning a new `__dict__` on a frozen instance raises an error.
#[test_case("Counter", false ; "annotated_dict_store")]
#[test_case("Any", true ; "unknown_class_dict_store")]
fn annotated_dict_store_keeps_frozen_unset(annotation: &str, fixed: bool) {
    let source = format!(
        "from dataclasses import dataclass\nfrom typing import Any\n\n\n@dataclass\nclass Counter:\n    count: int\n\n\
            \ndef reset(counter: {annotation}) -> None:\n    counter.__dict__ = {{\"count\": 0}}\n"
    );
    assert_eq!(
        fixed_source(&[("main.py", &source)], "dataclass-frozen", "main.py")
            .contains("@dataclass(frozen=True)\nclass Counter:"),
        fixed
    );
}

// A nested method's receiver belongs to the nested class. Captured variables can still refer to the outer dataclass.
#[test_case("    class Meta:\n        def mark(self) -> None:\n            self.flag = True\n", true ; "own_instance")]
#[test_case(
    "    class Meta:\n        def mark(self, outer) -> None:\n            outer.flag = 1\n", false
    ; "unannotated_parameter"
)]
#[test_case(
    "    def helper(self) -> None:\n        outer = self\n\n        class Marker:\n            def mark(self) -> None:\
        \n                outer.flag = 1\n\n        Marker().mark()\n",
    false
    ; "closure"
)]
#[test_case(
    "    def helper(self) -> None:\n        me = self\n\n        class Marker:\n            def mark(inner) -> None:\
        \n                me.flag = 1\n\n        Marker().mark()\n",
    false
    ; "closure_write_in_method_of_nested_class"
)]
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

#[test]
fn subclass_through_a_name_bound_to_a_class_of_the_same_file_keeps_frozen_unset() {
    let root = common::project(&[(
        "main.py",
        "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n\nBase = Release\n\n\
            \nclass Tagged(Base):\n    pass\n",
    )]);
    common::fix_unsafe(root.path(), "dataclass-frozen");
    assert!(!common::read(root.path(), "main.py").contains("frozen=True"));
}

#[test_case("from models import Config as C\n\nprint(C.timeout + 1)\n", false ; "import_alias")]
#[test_case(
    "from models import Config as config_class\n\nprint(config_class.timeout)\n", false
    ; "lowercase_import_alias"
)]
#[test_case("from models import Config\n\nklass = Config\nprint(klass.timeout)\n", false ; "name_bound_to_class")]
#[test_case(
    "from models import Config\n\n\nclass Sub(Config):\n    pass\n\n\nprint(Sub.timeout)\n", false
    ; "subclass"
)]
#[test_case("import models\n\nprint(type(models.Config()).timeout)\n", false ; "type_of_unresolved_object")]
#[test_case("import models\n\nprint(models.Config().__class__.timeout)\n", false ; "dunder_class_of_unresolved_object")]
#[test_case("import models\n\nprint(getattr(models.Config, \"timeout\"))\n", false ; "getattr_of_class")]
#[test_case("import models\n\nsetattr(models.Config, \"timeout\", 5)\n", false ; "setattr_of_class")]
#[test_case("from models import Config\n\nprint(Config().timeout)\n", true ; "instance_read")]
#[test_case("from models import Config\n\nprint(Config.name)\n", true ; "read_of_field_without_default")]
fn class_level_access_to_a_default_keeps_slots_unset(caller: &str, fixed: bool) {
    let source = fixed_source(
        &[
            (
                "models.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Config:\n    name: str\n    timeout: int = 30\
                    \n",
            ),
            ("main.py", caller),
        ],
        "dataclass-slots",
        "models.py",
    );
    assert_eq!(source.contains("slots=True"), fixed);
}

// A weak container needs its referent to support weak references. Adding slots alone can remove that support.
#[test_case(
    "import weakref\n\nfrom models import Release\n\nLIVE = weakref.WeakSet()\nLIVE.add(Release(\"1\"))\n", false
    ; "weak_set_of_another_module"
)]
#[test_case(
    "from weakref import WeakSet as Live\n\nfrom models import Release\n\nLIVE = Live()\nrelease = Release(\"1\")\
        \nLIVE.add(release)\n",
    false
    ; "aliased_weak_set"
)]
#[test_case(
    "from weakref import WeakKeyDictionary\n\nfrom models import Release\n\nNAMES = WeakKeyDictionary()\
        \nNAMES[Release(\"1\")] = 1\n",
    false
    ; "weak_key_dictionary"
)]
#[test_case(
    "from weakref import WeakValueDictionary\n\nfrom models import Release\n\nNAMES = WeakValueDictionary()\
        \nNAMES[\"one\"] = Release(\"1\")\n",
    false
    ; "weak_value_dictionary"
)]
#[test_case(
    "from weakref import WeakMethod\n\nfrom models import Release\n\nrelease = Release(\"1\")\nmethod = \
        WeakMethod(release.label)\n",
    false
    ; "weak_method"
)]
#[test_case(
    "import weakref\n\nfrom models import Release\n\nLIVE = weakref.WeakSet()\n\n\ndef keep(item) -> None:\n    \
        LIVE.add(item)\n",
    true
    ; "item_of_unknown_class"
)]
#[test_case("from models import Release\n\nNAMES = {}\nNAMES[\"one\"] = Release(\"1\")\n", true ; "no_weak_container")]
fn weak_container_holding_a_dataclass_keeps_slots_unset(caller: &str, fixed: bool) {
    let source = fixed_source(
        &[
            (
                "models.py",
                "from dataclasses import dataclass\n\n\n@dataclass\nclass Release:\n    version: str\n\n    def \
                    label(self) -> str:\n        return self.version\n",
            ),
            ("main.py", caller),
        ],
        "dataclass-slots",
        "models.py",
    );
    assert_eq!(source.contains("slots=True"), fixed);
}

// A class passed as `Annotated` metadata may be called later. An explicit constructor call in the annotation can be
// rewritten in place.
#[test_case("def load(path: Annotated[str, Release]) -> None:\n    print(path)\n", false ; "bare_class_in_metadata")]
#[test_case(
    "def load(path: Annotated[str, Release(\"where\")]) -> None:\n    print(path)\n", true
    ; "call_in_metadata"
)]
#[test_case(
    "def load(path: \"Annotated[str, Release('where')]\") -> None:\n    print(path)\n", true
    ; "call_in_string_annotation"
)]
#[test_case(
    "def load(path: \"Annotated[str, Release]\") -> None:\n    print(path)\n", false
    ; "bare_class_in_string_annotation"
)]
#[test_case(
    "def load(path: \"Annotated[str, make(Release)]\") -> None:\n    print(path)\n", false
    ; "class_passed_in_string_annotation"
)]
#[test_case("def load(path: Release(\"where\")) -> None:\n    print(path)\n", true ; "call_as_annotation")]
#[test_case("def load(path: Annotated[Release]) -> None:\n    print(path)\n", true ; "single_argument_annotated")]
#[test_case(
    "def load(path: \"Annotated[str, \" \"Release('where')]\") -> None:\n    print(path)\n", false
    ; "call_in_concatenated_string_annotation"
)]
fn annotated_metadata_keeps_keyword_only_fix_unsafe(annotation: &str, fixed: bool) {
    let root = common::project(&[("main.py", &annotated_release(annotation))]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    let text = common::read(root.path(), "main.py");
    assert_eq!(text.contains("@dataclass(kw_only=True)"), fixed, "{text}");
}

#[test_case(
    "def load(path: Annotated[str, Release(\"where\")]) -> None:\n    print(path)\n", "Release(name=\"where\")"
    ; "call_in_metadata"
)]
#[test_case(
    "def load(path: \"Annotated[str, Release('where')]\") -> None:\n    print(path)\n", "Release(name='where')"
    ; "call_in_string_annotation"
)]
#[test_case(
    "def load(path: Release(\"where\")) -> None:\n    print(path)\n", "Release(name=\"where\")"
    ; "call_as_annotation"
)]
fn annotated_call_is_rewritten_with_its_keyword(annotation: &str, rewritten: &str) {
    let root = common::project(&[("main.py", &annotated_release(annotation))]);
    common::fix_unsafe(root.path(), "dataclass-kw-only");
    let text = common::read(root.path(), "main.py");
    assert!(text.contains(rewritten), "{text}");
}

fn annotated_release(annotation: &str) -> String {
    format!(
        "from dataclasses import dataclass\nfrom typing import Annotated\n\n\n@dataclass\nclass Release:\n    name: str\
            \n\n\n{annotation}"
    )
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

#[test]
fn dataclass_counts_fields_annotated_by_name_only() {
    let root = common::project(&[
        ("pyproject.toml", "[tool.simplint.classes]\nmax-instance-attributes = 1\n"),
        (
            "module.py",
            "from dataclasses import dataclass\n\n\nclass Holder:\n    size = 0\n\n\n@dataclass\nclass Point:\n    \
             x: int\n    Holder.size: int = 1\n",
        ),
    ]);
    common::assert_report_count(root.path(), "SMP711", 0);
}
