import os

__all__ = [name for name in dir(os) if name.startswith("get")]


def dynamic_helper():
    return 1
