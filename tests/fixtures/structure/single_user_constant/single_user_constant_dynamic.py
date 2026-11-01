_LIMIT = 3
SHARED_LIMIT = 4


def check(value: int) -> bool:
    return value < _LIMIT < SHARED_LIMIT


__all__ = list(("check",))
