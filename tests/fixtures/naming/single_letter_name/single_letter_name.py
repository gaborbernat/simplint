import os as o
from os import path as p


def f(a: int, *, b: int = 1) -> int:
    x = a + b
    for i in range(3):
        x += i
    total = [y for y in range(x)]
    pairs = {k: v for k, v in {}.items()}
    try:
        pass
    except ValueError as e:
        raise RuntimeError from e
    with open(o.devnull) as h:
        h.read()
    if (n := len(total)) > 1:
        return n
    match pairs:
        case {"key": c}:
            return c
    key = lambda z: z
    _ = key
    return x


class C[T]:
    pass


def more(values: list[int]) -> list[int]:
    match values:
        case [first, *r]:
            return [first, *r]
    return [value for value in values if value]


def inside(values: list[int]) -> list[int]:
    return [y for v in values if (y := v * 2) if (lambda w: w)(y)]


class Encoder(JSONEncoder):
    @override
    def default(self, o: object) -> object:
        return o
