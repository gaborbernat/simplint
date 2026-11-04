class Reader:
    """Read files.

    :param source: Where to read from.
    """

    def __init__(self, source: str, limit: int) -> None:
        """Set up.

        :param source: The path to read.
        :param limit: How much to read.
        """
        self.source = source
        self.limit = limit


class Separate:
    """Hold a value."""

    def __init__(self, value: int) -> None:
        """Set up.

        :param value: The value.
        """
        self.value = value


class Undocumented:
    """Hold nothing.

    :param value: The value.
    """

    def __init__(self, value: int) -> None:
        self.value = value
