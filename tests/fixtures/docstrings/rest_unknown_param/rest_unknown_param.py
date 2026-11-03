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
