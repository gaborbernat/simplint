__all__ = ["ret_listed"]


def ret_listed(value: int) -> int:
    return value + 1


def ret_plain(value: int) -> int:
    return value + 2


ret_listed(1)
ret_plain(1)
