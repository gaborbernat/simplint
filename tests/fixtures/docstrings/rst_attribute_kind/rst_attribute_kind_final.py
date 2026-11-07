from typing import Final


class NodeFilter:
    """Filter verdicts.

    :cvar SHOW_ALL: Every node type.
    :ivar limit: Set by ``__init__``.
    """

    SHOW_ALL: Final = 0xFFFFFFFF
    limit: Final[int]

    def __init__(self) -> None:
        self.limit = 1


class Mislabeled:
    """Constants of the class.

    :ivar SHOW: A class constant, documented as an instance attribute.
    """

    SHOW: Final = 1
