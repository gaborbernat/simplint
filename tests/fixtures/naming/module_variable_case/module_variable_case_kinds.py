import os.path
from os.path import *

limit: int
limit = 3
limit: int

match os.sep:
    case "/" as captured:
        pass

type Pair[T] = tuple[T, T]


class Box[T]:
    size = limit
    scale = lambda self: self.size * limit


def first[T](items: list[T]) -> T:
    return items[limit]
