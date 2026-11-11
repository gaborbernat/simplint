import os

__all__ = [name for name in dir() if not name.startswith("_")]


def helper() -> str:
    return os.sep
