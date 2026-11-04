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


def middle(values: list[int]) -> None:
    """Check the values.

    :returns: Nothing at all.
    :raises ValueError: When one is negative.
    :rtype: None
    """
    if min(values) < 0:
        raise ValueError


def pragma(values: list[int]) -> int:
    """Add the values, which can be long enough to need the line length suppression after it."""  # noqa: E501
    return sum(values)


def pragma_closing() -> None:
    """Stop.

    :returns: Nothing at all."""  # noqa: E501


def not_a_generator(values: list[int]) -> list[int]:
    """Sort the values.

    :returns: The sorted values.
    :yields: Each value.
    """
    return sorted(values)


class Built:
    """Build things.

    :returns: A new builder.
    """

    @property
    def size(self) -> int:
        """The size.

        :returns: How many.
        :rtype: int
        """
        return 1
