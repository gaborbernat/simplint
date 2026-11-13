import abc
import contextlib
import enum
import functools
from dataclasses import dataclass
from functools import cache
from typing import override

import click
import pytest


def _doubled_value(value: int) -> int:
    """
    >>> _doubled_value(2)
    4
    """
    return value * 2


class _DoctestedBox:
    """Hold one value.

    >>> _DoctestedBox().value
    0
    """

    value = 0


@click.command()
def _greet_command() -> None:
    """Greet the world."""


@functools.partial(click.command, name="other")
def _other_command() -> None:
    """Greet the other world."""


@dataclass
class _PlainRecord:
    """Plain record."""

    name: str


class _Shelf:
    @property
    def size_of_shelf(self) -> int:
        """Shelf size."""
        return 1

    @size_of_shelf.setter
    def size_of_shelf(self, value: int) -> None:
        """Set shelf size."""

    @override
    def __len__(self) -> int:
        """Count the shelf."""
        return 0

    @staticmethod
    def _build_shelf() -> None:
        """Build shelf."""


@cache
def _cached_lookup() -> int:
    """Look it up."""
    return 1


@pytest.fixture
def _shelf_fixture() -> None:
    """Shelf fixture."""


@pytest.mark.parametrize("value", [1])
def _parametrized_shelf(value: int) -> None:
    """Check the shelf."""


class _AbstractShelf(abc.ABC):
    @abc.abstractmethod
    def _stock(self) -> int:
        """Count the stock."""


@contextlib.contextmanager
def _opened_shelf() -> object:
    """Open the shelf."""
    yield


@contextlib.asynccontextmanager
async def _opened_async_shelf() -> object:
    """Open the shelf later."""
    yield


@enum.unique
class _ShelfKind(enum.Enum):
    """Kinds of shelf."""

    WIDE = 1
