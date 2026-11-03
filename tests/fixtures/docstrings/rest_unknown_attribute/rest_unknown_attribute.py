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
