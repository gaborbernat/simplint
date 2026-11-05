import typing
from typing import TYPE_CHECKING

if TYPE_CHECKING:
    import collections.abc


def first(values: "collections.abc.Sequence[int]") -> int:
    return values[0]
