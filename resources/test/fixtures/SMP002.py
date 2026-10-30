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
