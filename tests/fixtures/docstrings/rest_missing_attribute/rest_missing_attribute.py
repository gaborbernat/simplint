from dataclasses import dataclass
from typing import ClassVar


class Engine:
    """Run things.

    :param speed: How fast.
    :ivar state: What it is doing.
    """

    limit = 3
    #: Documented where it is defined.
    name: ClassVar[str] = "engine"
    count: int

    def __init__(self, speed: int) -> None:
        self.speed = speed
        self.state = "idle"
        self._secret = 1
        if speed:
            self.started = True

    def reset(self) -> None:
        self.state = "idle"
        self.resets: int = 1

    @staticmethod
    def build() -> None: ...


@dataclass
class Point:
    """A point.

    :param x: Across.
    """

    x: int
    label: ClassVar[str] = "p"


class _Hidden:
    """Internal.

    :param value: The value.
    """

    def __init__(self, value: int) -> None:
        self.value = value
        self.extra = 1


class Summary:
    """Summarize."""

    total = 0


from typing import NamedTuple


class PairTuple(NamedTuple):
    """A pair.

    :param left: L.
    :param right: R.
    """

    left: int
    right: int


class Builder:
    """Build things."""

    @staticmethod
    def build(obj: object) -> None:
        obj.thing = 1

    def spread(*args: int) -> None: ...

    def collect(self) -> None:
        self.collect.cache_clear()
