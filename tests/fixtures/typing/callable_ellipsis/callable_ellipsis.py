import typing
from collections.abc import Callable

Handler = Callable[..., int]


def run(callback: typing.Callable[..., None], other: Callable[[int], None]) -> None:
    pass

ITEMS = list[int]
