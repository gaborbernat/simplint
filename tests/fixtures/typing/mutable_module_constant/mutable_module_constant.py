import os
import types
from collections.abc import Mapping, Sequence
from typing import Final, List

SUFFIXES = [".py", ".pyi"]
ONE = ["only"]
ONE_COMMA = ["only",]
SQUARES = [value * value for value in range(10)]
LETTERS = {"a", "b"}
EVENS = {value for value in range(10) if value % 2 == 0}
CODES = {"ok": 200}
INVERSE = {code: name for name, code in CODES.items()}
FROM_CALL = list(os.sep)
SET_CALL = set("abc")
DICT_CALL = dict(a=1)
_PRIVATE = ["x"]
lower: Final = ["y"]
TYPED: list[str] = ["z"]
TYPED_BARE: list = ["z"]
SET_BARE: set = {"z"}
FINAL_TYPED: Final[set[int]] = {1}
FINAL_DICT: Final[dict[str, int]] = {"a": 1}
COMMENTED: Final[
    list[
        int  # the comment stays
    ]
] = [1]
ABSTRACT: Sequence[str] = ["a"]
MAPPING: Mapping[str, int] = {"a": 1}
TYPING_LIST: List[str] = ["a"]
QUOTED: "list[str]" = ["a"]

CHANGED = []
SUBSCRIPT_WRITTEN = {}
DELETED = {"a": 1}
RETURNED = ["a"]
PASSED = ["a"]
REBOUND = ["a"]
REBOUND = ["b"]
AUGMENTED = ["a"]
lowercase = ["a"]
__all__ = ["reader"]
TUPLE = ("a",)
OTHER_CALL = sorted("abc")
COMPARED = ["a"]
ORDERED = ["a"]
MEMBER_OF = ["a"]
CONTAINER = ["a"]
SET_COMPARED = {"a"}
COMBINED = {"a"}
COPIED = ["a"]
SHOWN = {"a": 1}
CHECKED = ["a"]


def reader(key: str) -> object:
    for suffix in SUFFIXES:
        print(suffix)
    if ONE and len(ONE_COMMA) > 0:
        zip(SQUARES[0], [*LETTERS], {**CODES}, INVERSE.get(1))
    while key in EVENS:
        break
    assert FROM_CALL
    zip(sorted(SET_CALL), DICT_CALL.items(), _PRIVATE + lower, -len(TYPED), TYPED_BARE, SET_BARE)
    zip({item for item in FINAL_TYPED if item in FINAL_DICT}, ABSTRACT, MAPPING, TYPING_LIST, QUOTED)
    zip(",".join(SUFFIXES), not TYPED, CODES.keys() | {"x"})
    CHANGED.append(key)
    SUBSCRIPT_WRITTEN[key] = 1
    del DELETED["a"]
    PASSED.sort()
    zip(lowercase, TUPLE, OTHER_CALL, REBOUND)
    return RETURNED


def extend() -> None:
    global AUGMENTED
    AUGMENTED += ["b"]


def shadow() -> list[str]:
    LOCAL_ONLY = ["a"]
    return LOCAL_ONLY


def typed(key: str) -> bool:
    zip(COMPARED == [key], ORDERED < [key], MEMBER_OF in [[key]], SET_COMPARED == {key}, COMBINED | {key})
    zip(COPIED.copy(), isinstance(CHECKED, list))
    print(SHOWN)
    return key in CONTAINER


import pytest  # noqa: E402

pytestmark: Final[list[pytest.MarkDecorator]] = [pytest.mark.slow]
