import collections.abc
import typing
from collections.abc import Callable
from typing import Any

from mylib import Callable as Custom


def run(callback: Callable[..., object]) -> None:
    callback()


def apply(callback: Callable[[int], object], other: typing.Callable[[int], object]) -> None:
    callback(1)


def named(callback: collections.abc.Callable[[int], object]) -> None:
    callback(1)


def takes_object(callback: Callable[[object], None]) -> None:
    callback(1)


def returns_list(callback: Callable[..., list[object]]) -> None:
    callback()


def returns_any(callback: Callable[..., Any]) -> None:
    callback()


def custom(callback: Custom[..., object]) -> None:
    callback()
