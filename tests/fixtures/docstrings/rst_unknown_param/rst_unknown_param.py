from dataclasses import dataclass


def load(path: str, encoding: str) -> str:
    """Load a file.

    :param path: Where it is.
    :param mode: Gone since the last release.
    :type mode: str
    :param encoding: How to decode it,
        defaulting to UTF-8.
    """
    return path + encoding


def options(**settings: str) -> dict[str, str]:
    """Collect options.

    :keyword color: Any keyword is accepted.
    """
    return settings


class Reader:
    """Read files.

    :param source: Where to read from.
    :param limit: No longer taken.
    """

    def __init__(self, source: str) -> None:
        self.source = source

    def read(self, size: int) -> str:
        """Read some.

        :param self: Not documented.
        :param size: How much.
        """
        return self.source[:size]


@dataclass(frozen=True)
class Point:
    """A point.

    :param x: Across.
    :param z: Not a field.
    """

    x: int


class Inherited(Reader):
    """Read files.

    :param anything: The base class decides.
    """


def escaped(path: str) -> str:
    """Load a file\x2e

    :param missing: Left in place by the fix, as escapes make the text differ from the source.
    """
    return path


import attr


@attr.s
class Classic:
    """A classic attrs class.

    :param x: Across.
    :param y: Down.
    """

    x = attr.ib()
    y = attr.ib(default=0)


@dataclass
class Base:
    """A base.

    :param a: The first.
    """

    a: int


@dataclass
class Child(Base):
    """A child.

    :param a: The first, from the base.
    :param b: The second.
    """

    b: int


class Box:
    """Hold things.

    :param size: How big.
    :param gone: Removed long ago."""

    def __init__(self, size: int) -> None:
        self.size = size


from typing import overload


class SpanRange:
    """A span.

    :param start: Start.
    :param end: End.
    :param length: Length.
    """

    @overload
    def __init__(self, start: int, end: int) -> None: ...

    @overload
    def __init__(self, *, length: int) -> None: ...

    def __init__(self, start: int = 0, end: int = 0, *, length: int = 0) -> None:
        self.start = start
        self.end = end or start + length
