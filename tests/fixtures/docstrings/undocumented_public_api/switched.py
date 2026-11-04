import sys

if sys.platform == "win32":

    def open_thing() -> int: ...

else:

    def open_thing() -> int: ...


try:
    from _accel import fast
except ImportError:

    def fast(data: bytes) -> int:
        """Hash the data in pure Python, slower than the C version."""
        return len(data)


from typing import overload


class Versioned:
    """Change with the version."""

    if sys.version_info >= (3, 12):

        def method(self, value: int) -> int:
            """Return the value, unchanged on new versions."""
            return value


@overload
def convert(value: int) -> int:
    """Convert an integer, keeping the type."""


@overload
def convert(value: str) -> str: ...


def convert(value: int | str) -> int | str:
    """Convert a value."""
    return value
