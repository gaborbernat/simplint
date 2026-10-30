import os

limit = 3
_timeout = 1
PUBLIC_VALUE = 2
logger = os.getenv("X")
Alias = int
pytestmark = []


def use() -> int:
    return limit + _timeout + PUBLIC_VALUE


__all__ = ["PUBLIC_VALUE", "logger"]
