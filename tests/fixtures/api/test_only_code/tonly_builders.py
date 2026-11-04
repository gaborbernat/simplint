def build_tonly_fixture() -> int:
    return 1


def shared_tonly() -> int:
    return 2


def unused_tonly() -> int:
    return 3


def recurse_tonly(depth: int) -> int:
    return recurse_tonly(depth - 1) if depth else 0


def own_tonly() -> int:
    return 4


def uses_own_tonly() -> int:
    return own_tonly()


def _private_tonly() -> int:
    return 5


def register(function):
    return function


@register
def registered_tonly() -> int:
    return 6


class TonlyRecord:
    pass


class TonlyShared:
    pass


class TonlyPublished:
    pass


class TonlyUsed:
    pass


__all__ = ["TonlyPublished"]
