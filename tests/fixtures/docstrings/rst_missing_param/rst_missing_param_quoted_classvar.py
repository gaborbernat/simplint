from dataclasses import dataclass
from typing import ClassVar


@dataclass
class Point:
    """A point.

    :param x: Across.
    """

    x: int
    count: "ClassVar[int]" = 0
