import sys


def handle_a() -> int:
    return 1


def dispatch(kind: str) -> int:
    return getattr(sys.modules[__name__], "handle_" + kind)()


__all__ = ["dispatch"]
