from typing import ClassVar


class Engine:
    """Run jobs.

    :ivar count: How many runs.
    """

    count: "ClassVar[int]" = 0
