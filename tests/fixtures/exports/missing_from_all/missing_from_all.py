LIMIT = 3


def shared() -> int:
    return LIMIT


def local() -> int:
    return 1


__all__ = ["local"]
