from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from os import PathLike

if TYPE_CHECKING:
    from collections.abc import Sequence


def first(values: "Sequence[int]", path: "PathLike") -> int:
    return values[0]
