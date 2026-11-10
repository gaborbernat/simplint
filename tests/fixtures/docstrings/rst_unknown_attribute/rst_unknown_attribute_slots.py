class Bounds:
    """Limits of a range.

    :cvar low: The lower limit.
    :cvar high: The upper limit.
    :ivar label: The name.
    :ivar missing: Not here.
    """

    __slots__ = ("label",)
    low, high = 0, 10


class Named:
    """A name.

    :ivar text: The text.
    """

    __slots__ = "text"


class Listed:
    """Some names.

    :ivar first: The first.
    :ivar second: The second.
    """

    __slots__ = ["first", "second"]


class Keyed:
    """Keys.

    :ivar key: The key.
    """

    __slots__ = {"key": "The key."}


class Split:
    """Split a sequence.

    :cvar head: The first item.
    :cvar tail: The rest.
    """

    head, *tail = 1, 2, 3


class Table:
    """A table.

    :cvar entries: The entries.
    """

    entries = {}
    entries["key"] = 1
