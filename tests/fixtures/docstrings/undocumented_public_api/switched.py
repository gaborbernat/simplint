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


if sys.version_info >= (3, 12):
    LIMIT = 10
    """The most a call takes on new versions."""
else:
    LIMIT = 5
    """The most a call takes on old versions."""

try:
    TIMEOUT = float(sys.argv[1])
    """Seconds to wait, as given."""
except IndexError:
    TIMEOUT = 1.0


match sys.platform:
    case "win32":

        def open_match() -> int: ...

        class Platform:
            def name(self) -> str: ...
    case _:

        def _private_match() -> int:
            """Restates the signature."""
            return 1
