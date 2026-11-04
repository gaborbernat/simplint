from collections.abc import Iterator


def scale(value: float, factor) -> float:
    """Scale a value.

    :param value: The value.
    :type value: float
    :param factor: The factor.
    :type factor: float
    :rtype: float
    """
    return value * factor


def inline(value: float, factor) -> None:
    """Scale a value.

    :param float value: The value.
    :param float factor: The factor.
    """


def untyped(value):
    """Scale a value.

    :param value: The value.
    :returns: The scaled value.
    :rtype: float
    """
    return value


def counted() -> Iterator[int]:
    """Count.

    :yields: The numbers.
    :ytype: int
    """
    yield 1


class Scaler:
    """Scale values.

    :param factor: The factor.
    :type factor: float
    :ivar int calls: How often it ran.
    :vartype limit: int
    """

    limit = 3

    def __init__(self, factor: float) -> None:
        self.factor = factor
        self.calls = 0
