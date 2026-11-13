import typing
from collections.abc import Callable

Handler = Callable[..., int]


def run(callback: typing.Callable[..., None], other: Callable[[int], None]) -> None:
    pass

ITEMS = list[int]

F = typing.TypeVar("F", bound=Callable[..., object])
G = typing.TypeVar("G", Callable[..., int], int)


def mark[W: Callable[..., object]](func: W) -> W:
    return func


def apply[W: Callable[..., object], **P](func: Callable[..., int]) -> None:
    pass


class Box[W: Callable[..., object]]:
    pass


type Wrapper[W: Callable[..., object]] = list[W]
