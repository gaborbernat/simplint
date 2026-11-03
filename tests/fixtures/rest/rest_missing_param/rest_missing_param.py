def copy(source: str, target: str, *args: str, **kwargs: str) -> None:
    r"""Copy a file.

    :param source: Where from.
    :param \*args: More sources.
    """


def summary(source: str) -> None:
    """Copy a file."""


def _helper(source: str) -> None:
    """Copy a file.

    :param other: Unrelated.
    """


class Copier:
    """Copy files.

    :param source: Where from.
    """

    def __init__(self, source: str, target: str) -> None:
        """Set up.

        :param target: Where to.
        """
        self.source = source
        self.target = target

    def run(self, verbose: bool) -> None:
        """Run it."""

    @staticmethod
    def check(path: str) -> None:
        """Check a path."""


class Bare:
    """Copy files."""

    def __init__(self, _: str, quiet: bool) -> None:
        self.quiet = quiet
