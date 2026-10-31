def shared_multiline() -> int:
    return 1


def other() -> int:
    return 2


_PRIVATE = 1

__all__ = [
    "other",
]
