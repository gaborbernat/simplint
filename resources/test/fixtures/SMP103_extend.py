def listed() -> int:
    return 1


def extended() -> int:
    return 2


def appended() -> int:
    return 3


def forgotten() -> int:
    return 4


__all__ = ["listed"]
__all__.extend(["extended"])
__all__.append("appended")
