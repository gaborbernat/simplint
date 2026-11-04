import collections
import functools
import pathlib
import json
import os
import os.path
import platform
import string
import sys
import tempfile
import uuid
from collections.abc import Sequence
from dataclasses import dataclass
from pathlib import Path
from typing import Final, Optional

KIND_SEPARATOR: Final = "-"
KIND_NAMES: list[str] = ["a", "b"]
KIND_COUNTS = (1, 2)
KIND_TWICE = 1
KIND_TWICE = "x"


@dataclass
class KindRecord:
    label: str
    count: int
    extra: object

    @property
    def title(self) -> str:
        return self.label.title()

    def shout(self) -> str:
        return self.label.upper()

    def describe(self):
        return self.label


class KindOdd:
    def __new__(cls):
        return "odd"


class KindMeta(metaclass=type):
    pass


@functools.total_ordering
class KindOrdered:
    label: str


@some_decorator
class KindDecorated:
    label: str


def kind_text() -> str:
    return "t"


async def kind_async() -> str:
    return "t"


@functools.cache
def kind_cached() -> str:
    return "t"


@some_decorator
def kind_decorated() -> str:
    return "t"


def kind_untyped():
    return "t"


def kind_literals(data: bytes, count: int, raw: "int", other: "KindRecord") -> list[object]:
    return [
        data.decode() + "!",
        "-" * count + "!",
        count * "-" + "!",
        "%s items" % count + "!",
        "a" + ("b" if count else "c"),
        "a" + (count and "b"),
        "a" + (text := "b"),
        "a" + "abc"[0],
        "a" + ["x", "y"][0],
        "a" + ["x", "y"][:1],
        "a" + [1, 2][0],
        "a" + [][0],
        "a" + ["x", 1][0],
        "a" + {"x"}.pop(),
        "a" + str(count * 2 - 1 // 1 % 3),
        "a" + chr(raw),
        "a" + other.label,
    ]


def kind_calls(record: KindRecord, names: Sequence[str], maybe: str | None, alias: Optional[str]) -> list[object]:
    for name in names:
        pass
    for position in range(3):
        pass
    return [
        "a" + name,
        "a" * position + "!",
        "a" + sorted(names)[0],
        "a" + list(names, key=len)[0],
        "a" + list(KIND_COUNTS)[0],
        "a" + list(record)[0],
        "a" + str(len(names)),
        "a" + kind_text(),
        "a" + kind_async(),
        "a" + kind_cached(),
        "a" + kind_decorated(),
        "a" + kind_untyped(),
        "a" + KindRecord("x", 1, None).label,
        "a" + KindRecord("x", 1, None).title,
        "a" + KindRecord("x", 1, None).shout(),
        "a" + KindRecord("x", 1, None).describe(),
        "a" + KindRecord("x", 1, None).extra,
        "a" + KindRecord("x", 1, None).count,
        "a" + KindOdd().label,
        "a" + KindMeta().label,
        "a" + KindOrdered().label,
        "a" + KindDecorated().label,
        "a" + record.missing,
        "a" + record.shout,
        "a" + (maybe or "none"),
        "a" + (alias or "none"),
        "a" + (maybe or alias),
        "a" + {"k": "v"}.get("k"),
        "a" + {"k": "v"}.get("k", "d"),
        "a" + {"k": "v"}.get("k", 1),
        "a" + {"k": "v"}.get(),
        "a" + {"k": "v"}.keys()[0],
        "a" + KIND_NAMES[0],
        "a" + KIND_SEPARATOR,
        "a" + KIND_TWICE,
    ]


def kind_library(path: Path, raw: bytes) -> list[object]:
    with tempfile.TemporaryDirectory() as folder, open(path) as handle:
        pass
    with tempfile.NamedTemporaryFile() as named:
        pass
    with kind_text as called:
        pass
    return [
        "a" + os.getcwd(),
        "a" + platform.system(),
        "a" + json.dumps([]),
        "a" + os.path.join(folder, "x"),
        "a" + os.path.join(raw, b"x"),
        "a" + os.path.splitext("x.py")[0],
        "a" + os.path.splitext(raw)[0],
        "a" + os.path.splitext()[0],
        "a" + os.environ["HOME"],
        "a" + os.environ.get("HOME", "/"),
        "a" + os.getenv("HOME", 1),
        "a" + (os.getenv("HOME") or "/"),
        "a" + os.getenv(),
        "a" + os.sep + sys.platform + string.digits,
        "a" + os.missing,
        "a" + uuid.uuid4().hex,
        "a" + uuid.UUID(int=1).hex,
        "a" + kind_text().hex,
        "a" + path.name + path.parent.stem + path.parent.parent.name,
        "a" + path.owner,
        "a" + Path("x").with_suffix(".y").as_posix(),
        "a" + (Path.cwd() / "x").name,
        "a" + type(path).__name__,
        "a" + __file__ + __name__,
        "a" + __doc__,
        "a" + handle,
        "a" + named,
        "a" + called,
        "a" + (lambda: "x")(),
    ]


def kind_local_import() -> str:
    import os

    return "a" + os.sep


def kind_cycles(items: list[str], counts: list[int]) -> list[object]:
    text = "a"
    text = text + "!"
    total = 0
    total += 1
    loop = loop + "!"
    mixed = "a"
    mixed = 1
    left, right = "a,b".split(",")
    pair = ("a", 1)
    first, second = pair
    return [
        "a" + text,
        "a" * total + "!",
        "a" + loop,
        "a" + mixed,
        "a" + left + right,
        "a" + first,
        "a" + [item for item in items][0],
        ["a" + item for item in items],
        ["a" * count for count in counts],
        ["a" + key for key, _ in items],
    ]


class KindHolder:
    k_prefix = "p"
    k_declared: str
    k_suffix: Final = "s"
    k_looped = ""
    for k_looped in "ab":
        pass
    k_combined = "a" + k_prefix

    def __init__(self, name: str) -> None:
        self.k_name = name
        self.k_label: str = name
        self.k_joined = "a"
        self.k_joined = self.k_joined + "b"
        self.k_other = "x"

    @property
    def k_shown(self) -> str:
        return self.k_name

    def render(self) -> str:
        return self.k_prefix

    def build(self) -> list[object]:
        return [
            "a" + self.k_prefix,
            "a" + self.k_declared,
            "a" + self.k_suffix,
            "a" + self.k_name,
            "a" + self.k_label,
            "a" + self.k_joined,
            "a" + self.k_shown,
            "a" + self.render(),
            "a" + self.render,
            "a" + self.k_looped,
            "a" + self.missing,
            "a" + self.k_other,
        ]

    @staticmethod
    def static(self) -> str:
        return "a" + self.k_prefix

    def reader(this, self: str) -> str:
        return "a" + self.k_prefix + this.k_prefix

    def rebinds(self) -> str:
        self = KindHolder("x")
        return "a" + self.k_prefix

    def nested(self) -> str:
        def inner() -> None:
            self.k_later = "x"

        self.k_packed, _ = "a", "b"
        return "a" + self.k_later + self.k_packed

    def deleted(self) -> str:
        del self.k_gone
        return "a" + self.k_gone

    def typed(self) -> str:
        self.k_marked: object = 1
        return "a" + self.k_marked


def kind_outside(holder: KindHolder) -> str:
    holder.k_other = 1
    return "a" + holder.k_prefix


class KindBase:
    k_value = "v"

    def show(self) -> str:
        return "a" + self.k_value


class KindChild(KindBase):
    pass


def kind_annotations(
    count: "int",
    data: "bytes",
    ints: list[int],
    pairs: dict[str, int],
    tagged: Final[str],
    unknown: set[object],
    other: frozenset,
    shadowed: Sequence,
) -> list[object]:
    return [
        "a" * count + "!",
        "a" + data.hex(),
        "a" * ints[0] + "!",
        "a" + pairs["k"],
        "a" + tagged,
        "a" + unknown.pop(),
    ]


@functools.lru_cache(maxsize=1)
def kind_bounded() -> str:
    return "t"


@dataclass(frozen=True)
class KindFrozen:
    label: str


class KindAgain:
    pass


class KindAgain:
    pass


def kind_more(
    data: bytes,
    count: int,
    items: list[str],
    mapping: dict[str, str],
    kind_type: type[str],
    ordered: collections.OrderedDict[str, str],
    chunks: list[bytes],
    again: KindAgain,
    location: pathlib.Path,
) -> list[object]:
    seen = {item for item in items if item}
    keyed = {item: item for item in items}
    twice = "x"
    return [
        "a" + data[0],
        "a" + mapping.get("k"),
        "a" + mapping.get("k", "d", "e"),
        "a" + mapping.get("k", "d"),
        "a" + {"k": "v"}["k"],
        "a" + {1: "v"}[1],
        "a" + {}.get("k", "d"),
        "a" + {**mapping}["k"],
        "a" + (["x"] * 2)[0],
        "a" * (count - 1) + "!",
        "a" + (count / 2),
        "a" * "abc".count("a") + "!",
        "a" + "x".encode().decode(),
        "a" + kind_bounded(),
        "a" + KindFrozen("x").label,
        "a" + kind_type,
        "a" + ordered["k"],
        "a" + chunks[0],
        "a" + again,
        "a" + location.name,
        "a" + seen.pop() + keyed["k"],
        {"a" + item for item in items},
        {item: "a" + item for item in items},
        ["a" + rest for *rest, last in items],
        ["a" + item for item in items for item.attr in items],
        "a" + item + item,
        "a" + twice + twice,
        "a" * [1, 2][:1][0] + "!",
        "a" + (2 * ["x"])[0],
    ]


# Each name reads itself, so finding its kind checks the one before twice: a long chain must stay quick.
def kind_chain() -> str:
    chain0 = "a"
    chain0 += chain0
    chain1 = chain0
    chain1 += chain1
    chain2 = chain1
    chain2 += chain2
    chain3 = chain2
    chain3 += chain3
    chain4 = chain3
    chain4 += chain4
    chain5 = chain4
    chain5 += chain5
    chain6 = chain5
    chain6 += chain6
    chain7 = chain6
    chain7 += chain7
    chain8 = chain7
    chain8 += chain8
    chain9 = chain8
    chain9 += chain9
    chain10 = chain9
    chain10 += chain10
    chain11 = chain10
    chain11 += chain11
    chain12 = chain11
    chain12 += chain12
    chain13 = chain12
    chain13 += chain13
    chain14 = chain13
    chain14 += chain14
    chain15 = chain14
    chain15 += chain15
    chain16 = chain15
    chain16 += chain16
    chain17 = chain16
    chain17 += chain17
    chain18 = chain17
    chain18 += chain18
    chain19 = chain18
    chain19 += chain19
    chain20 = chain19
    chain20 += chain20
    chain21 = chain20
    chain21 += chain21
    chain22 = chain21
    chain22 += chain22
    chain23 = chain22
    chain23 += chain23
    chain24 = chain23
    chain24 += chain24
    chain25 = chain24
    chain25 += chain25
    chain26 = chain25
    chain26 += chain26
    chain27 = chain26
    chain27 += chain27
    chain28 = chain27
    chain28 += chain28
    chain29 = chain28
    chain29 += chain29
    chain30 = chain29
    chain30 += chain30
    chain31 = chain30
    chain31 += chain31
    chain32 = chain31
    chain32 += chain32
    chain33 = chain32
    chain33 += chain33
    chain34 = chain33
    chain34 += chain34
    chain35 = chain34
    chain35 += chain35
    chain36 = chain35
    chain36 += chain36
    chain37 = chain36
    chain37 += chain37
    chain38 = chain37
    chain38 += chain38
    chain39 = chain38
    chain39 += chain39
    return "a" + chain39
