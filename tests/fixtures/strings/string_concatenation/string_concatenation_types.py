from os import pathsep
from pathlib import Path
from typing import Final

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
