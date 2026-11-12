import heapq

_QUEUE = []


def push(item: int) -> int:
    heapq.heappush(_QUEUE, item)
    return len(_QUEUE)


_REGISTRY = {}


def registry() -> dict[str, int]:
    return _REGISTRY


_ALIASED = [1]


def alias() -> list[int]:
    aliased = _ALIASED
    return aliased


_YIELDED = {"a": 1}


def produce() -> object:
    yield _YIELDED


_NESTED = {"a": [1]}


def nested() -> list[int]:
    return _NESTED["a"]


_ROWS = ([1], [2])


def rows() -> int:
    total = 0
    for row in _ROWS:
        total += sum(row)
    return total


_PAIRS = [[1, 2]]


def has_pair(item: list[int]) -> bool:
    return item in _PAIRS and len(_PAIRS) > 0


_NAMES = ["a", "b"]


def others(name: str) -> list[str]:
    ordered = sorted(_NAMES)
    return [*ordered, *(item for item in _NAMES if item != name), _NAMES[0], str(_NAMES.index(name))]


_LIMITS = {"low": 1, "high": 2}


def total_limit(key: str) -> int:
    total = sum(_LIMITS.values()) + max(_LIMITS)
    for limit in _LIMITS:
        total += len(limit)
    return total + _LIMITS.get(key, 0) + len(list(_LIMITS.items()))


_DEFAULTS = {"a": 1}
_PARTS = ["a"]


def merged(extra: dict[str, int]) -> tuple[dict[str, int], list[str]]:
    return {**_DEFAULTS, **extra}, [*_PARTS, "b"]


_REPEATED = [1] * 3


def repeated() -> list[int]:
    return _REPEATED


_SQUARES = [number * number for number in range(3)]
_LETTERS = {letter for letter in "ab"}
_WIDTHS = {letter: len(letter) for letter in "ab"}


def comprehended() -> object:
    return _SQUARES, _LETTERS, _WIDTHS
