class Engine:
    """Run things.

    :ivar state: What it is doing.
    :ivar ghost: Removed long ago.
    :cvar limit: The most it takes.
    :vartype gone: int
    :ivar size: The size property.
    """

    limit = 3

    def __init__(self) -> None:
        self.state = "idle"

    @property
    def size(self) -> int:
        """The size."""
        return 1


class Derived(Engine):
    """Run more things.

    :ivar inherited: Comes from the base.
    """


class Typed[T]:
    """Hold a value.

    :ivar missing: Not here.
    """


class Pair:
    """A pair of values.

    :ivar left: The first.
    :ivar right: The second.
    :ivar handle: An open file.
    :ivar rest: What is left.
    :ivar cursor: Where it is.
    :ivar total: The sum.
    """

    def __init__(self) -> None:
        self.left, (self.right, *self.rest) = 1, (2, 3)
        with open("x") as self.handle:
            pass
        for self.cursor in range(3):
            pass
        self.total += 1
        local = 1
