from os import pathsep
from pathlib import Path
from typing import Annotated, Final

from markupsafe import Markup
from pypdf import PageRange

CONCAT_SEPARATOR = "-"
CONCAT_PREFIX: Final[str] = "pre"
concat_counter = 0


def concat_opaque(value, path: Path, items: list[str]) -> list[object]:
    return [
        PageRange(slice(0, 5)) + "2:7",
        Markup("foo") + "bar",
        value + "",
        path + pathsep + "foo",
        "a" + items[0],
        "a" + concat_counter,
        "a" + len(items),
    ]


def concat_known(label: str, items: list[str], flag: bool) -> list[str]:
    built = "start"
    built += label
    joined = ", ".join(items)
    return [
        label + "!",
        built + "?",
        joined + ".",
        "ab".replace("a", "c") + label + ".",
        (label if flag else "none") + "x",
        label.upper() + "-" + label[0],
        "<" + CONCAT_PREFIX + label,
        "<" + CONCAT_SEPARATOR + label,
        "n=" + str(flag),
    ]


def concat_rebound(label: str, items: list[str]) -> list[str]:
    for label in items:
        pass
    for label in range(3):
        pass
    looped = [label + "!" for label in items]
    shadow = "x"
    shadow = len(items)
    return [label + "?", *looped, "s" + shadow]


def concat_shadowed_builtin(value: int) -> str:
    str = repr
    return "v=" + str(value)


def concat_global(text: str) -> str:
    def reset() -> None:
        global CONCAT_SEPARATOR
        CONCAT_SEPARATOR = 1

    reset()
    return CONCAT_SEPARATOR + text


class ConcatHolder:
    suffix = "!"
    combined = "a" + suffix

    def describe(self, suffix=suffix + "?") -> str:
        return "b" + suffix


concat_lambda = lambda part: "x" + part


def concat_forms(
    label: str,
    quoted: "str",
    tagged: Annotated[str, "tag"],
    numbered: Annotated[int, "n"],
    maybe: str | None,
    listed: list[str],
) -> list[object]:
    annotated: object = "x"
    prefixed = "b" + label
    repeated = "ab"
    repeated *= 2
    try:
        pass
    except ValueError as caught:
        pass
    match label:
        case "a":
            pass
        case str() as matched:
            pass
        case [*rest]:
            pass
        case {**extra}:
            pass
    return [
        "a" + prefixed,
        (label or "none") + "x",
        "a" + listed.pop(),
        "a" + (lambda: label)(),
        "a" + concat_undefined,
        "a" + annotated,
        "a" + repeated,
        "a" + caught,
        "a" + matched,
        quoted + "!",
        tagged + "!",
        numbered + "!",
        maybe + "!",
    ]


def concat_declared_global() -> str:
    global concat_counter
    return "a" + concat_counter


def concat_comprehension(label: str, listed: list[str]) -> list[str]:
    return ["a" + label for label in listed]


def concat_nested_sum(label: str) -> str:
    return "a" + ("b" + label)
