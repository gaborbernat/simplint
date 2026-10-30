from typing import Final

_TIMEOUT: Final[int] = 30
_RETRIES = 3
_SHARED = 1
# cached: rebuilding the table on every call is too slow
_TABLE = {"a": 1}
PUBLIC = 5


def connect() -> int:
    return _TIMEOUT


def retry() -> int:
    return _RETRIES + _RETRIES + _TABLE["a"]


def first() -> int:
    return _SHARED


def second() -> int:
    return _SHARED + PUBLIC


_PATTERN = "x"
_MULTILINE = (
    "a"
    "b"
)


def loop(items: list[str]) -> list[str]:
    return [item for item in items if item == _PATTERN]


def multi() -> str:
    return _MULTILINE


_REPEAT = 2
_LOOPED = "z"


def twice(items: list[str]) -> list[str]:
    return [item for item in items if item == _LOOPED][:_REPEAT] * _REPEAT
