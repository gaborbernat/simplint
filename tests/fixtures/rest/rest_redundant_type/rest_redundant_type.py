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
    :rtype: None
    """


class Scaler:
    """Scale values.

    :param factor: The factor.
    :type factor: float
    """

    def __init__(self, factor: float) -> None:
        self.factor = factor
