def documented() -> None:
    """Explain the obvious."""
    print("x")


def only_docstring() -> None:
    """Nothing else."""


class Thing:
    """A thing."""

    def method(self) -> None:
        """Do it."""
        return None


def plain() -> None:
    return None


def inline() -> None: """Same line."""; print("y")


def one_liner() -> None: """Whole body."""


def shared_line() -> None:
    """Shares its line."""; print("z")


def commented() -> None:
    """Keeps the comment.""";  # why
    print("w")


def pragma() -> int:
    """A docstring this line-length suppression was written for, and nothing else at all."""  # noqa: E501


def commented() -> int:
    """Kept comment."""  # a plain comment stays
