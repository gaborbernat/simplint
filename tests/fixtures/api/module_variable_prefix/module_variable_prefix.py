import os

limit = 3
_timeout = 1
PUBLIC = 2
_LOGGER = os.getenv("X")
_listed = 4
__hidden = 5
Alias = int
pytestmark = []


def use() -> int:
    return limit + _timeout + PUBLIC + _listed + __hidden


__all__ = ["_listed", "_LOGGER", "PUBLIC", "__hidden"]
