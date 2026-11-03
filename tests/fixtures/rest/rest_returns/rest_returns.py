from abc import ABC, abstractmethod
from collections.abc import Iterator
from typing import NoReturn


def total(values: list[int]) -> int:
    """Add the values."""
    return sum(values)


def untyped(values):
    """Add the values."""
    return sum(values)


def nothing(values: list[int]) -> None:
    """Print the values.

    :returns: Nothing at all.
    :rtype: None
    """
    print(values)


def implicit(values):
    """Print the values.

    :returns: Nothing at all.
    """
    print(values)
    return


def placeholder_pass(values):
    """Print the values.

    :returns: What subclasses decide.
    """
    pass


def placeholder_raise(values):
    """Print the values.

    :returns: What subclasses decide.
    """
    raise NotImplementedError


def assigned(values):
    """Keep the values.

    :returns: Nothing at all.
    """
    kept = values


def stop() -> NoReturn:
    """Stop.

    :returns: Never.
    """
    raise SystemExit


def numbers() -> Iterator[int]:
    """Count.

    :returns: The numbers.
    """
    yield 1


def silent() -> Iterator[int]:
    """Count."""
    yield 1


def counted() -> Iterator[int]:
    """Count.

    :yields: The numbers.
    """
    yield from range(3)


def _private() -> int:
    """Add the values."""
    return 1


class Shape(ABC):
    """A shape."""

    @abstractmethod
    def area(self):
        """The area.

        :returns: The area.
        """
        ...

    @property
    def name(self) -> str:
        """The name."""
        return "shape"

    def __init__(self) -> None:
        """Set up.

        :returns: Nothing.
        """
