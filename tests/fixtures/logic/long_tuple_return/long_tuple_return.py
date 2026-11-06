from typing import Sequence, Tuple


def pair() -> tuple[int, int]:
    return 1, 2


def triple() -> tuple[int, int, int]:
    return 1, 2, 3


def parenthesized(flag: bool) -> tuple[str, str, str]:
    if flag:
        return ("a", "b", "c")
    return ("d", "e", "f")


def starred(items: list[int]) -> tuple[int, ...]:
    return (*items, 1, 2)


def nested() -> int:
    def inner() -> tuple[int, int, int]:
        return 1, 2, 3

    return inner()[0]


class Store:
    def __reduce__(self) -> tuple[type, tuple[int, int, int]]:
        return type(self), (1, 2, 3)

    def __getnewargs__(self) -> tuple[int, int, int]:
        return 1, 2, 3

    def values(self) -> tuple[int, int, int, int]:
        return 1, 2, 3, 4

    def __getstate__(self) -> tuple[int, int, int]:
        return 1, 2, 3

    def _sort_key(self) -> tuple[int, int, int]:
        return 1, 2, 3

    def key(self) -> tuple[int, int, int]:
        return 1, 2, 3


def integers() -> tuple[int, ...]:
    return 0, 1, -1


def typing_integers() -> Tuple[int, ...]:
    return 0, 1, -1


def fixed() -> tuple[int, int, int]:
    return 0, 1, -1


def unannotated():
    return 0, 1, -1


def generic() -> Sequence[int]:
    return 0, 1, -1
