"""Module docstring is not a comment."""

import math

"""Area of the unit circle."""
AREA = math.pi

"""Attribute docstrings follow an assignment."""
TOTAL = 1


class Config:
    """Class docstring."""

    timeout = 3
    """Seconds to wait for the server."""
    retries: int
    """Retries after a failure."""

    def run(self) -> None:
        """Function docstring."""
        start()
        """Second string in a body is a comment."""
        wait()


def sole() -> None:
    if math.pi:
        """A sole statement cannot leave its body empty."""
    elif AREA:
        """Two strings cannot become comments together,"""
        """since the body would be left empty."""


def shared() -> None:
    start(); """shares its line"""
    return None


def multi() -> None:
    start()
    """Start the worker.
    Then wait for it."""
    wait()


def empty() -> None:
    start()
    """"""
    wait()


def directive() -> None:
    start()
    """fmt: off"""
    wait()


def concatenated() -> None:
    start()
    ("""first part """
     """second part""")
    wait()


def bang() -> None:
    start()
    """!not a shebang"""
    wait()


def wide() -> None:
    start()
    """This string has a line that is far too long to fit in the comment width of the file, so no fix is made."""
    wait()


def raw_directive() -> None:
    start()
    """type: ignore"""
    wait()


def conditional(flag: bool) -> None:
    if flag:
        start()
    else:
        """Else branches are bodies too."""
        wait()


def escapes() -> None:
    start()
    """tab\tinside"""
    wait()


def trailing() -> None:
    start()
    """text"""  # note
    wait()


def marker() -> None:
    start()
    "single quotes mark a line, not a comment"
    wait()
