from typing import Callable, TypeAlias


type Vector = list[float]
Shape: TypeAlias = tuple[Vector, int]
Pair = Shape | None
Left = Right = int
_Hidden = int


class Frame[T: Shape = Shape]:
    tag: Pair

    def cells(self) -> list[Vector]:
        return []

    def _pick(self) -> _Hidden:
        return 0


def run[**P](call: Callable[P, int]) -> int:
    return call()


def scan(size: "Left", hidden: _Hidden) -> Frame[int]:
    return Frame()


class Tree:
    def children(self) -> list[Tree]:
        return []


__all__ = ["Frame", "Pair", "Shape", "Tree", "Vector", "run", "scan"]
