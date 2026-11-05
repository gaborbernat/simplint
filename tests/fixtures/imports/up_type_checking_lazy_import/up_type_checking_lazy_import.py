from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from collections.abc import Sequence
    from os import PathLike
    from . import helper


def first(values: "Sequence[int]", path: "PathLike") -> int:
    return values[0]
