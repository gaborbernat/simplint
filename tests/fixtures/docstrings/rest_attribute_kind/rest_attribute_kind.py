from typing import ClassVar


class Engine:
    """Run jobs.

    :ivar limit: The most it runs at once.
    :cvar state: What it is doing.
    :cvar name: Its name.
    :var count: How many ran.
    :ivar mode: The mode, with a class default.
    :cvar label: The label.
    :ivar ghost: Not an attribute.
    """

    limit = 3
    name: ClassVar[str] = "engine"
    count = 0
    mode = "fast"
    label: ClassVar[str] = "engine"

    def __init__(self) -> None:
        self.state = "idle"
        self.mode = "slow"
