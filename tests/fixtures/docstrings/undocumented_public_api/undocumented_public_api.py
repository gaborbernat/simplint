from typing import overload

#: Seconds to wait for a response.
TIMEOUT = 30
RETRIES = 3
"""How often a failed request is retried."""
LIMIT = 5
WINDOW: int = 10
NAME = "demo"  #: The name the plugin registers under.


def fetch(url: str) -> str:
    """Download a resource."""
    return url


def undocumented(url: str) -> str:
    return url


@overload
def parse(value: int) -> int: ...
@overload
def parse(value: str) -> str: ...
def parse(value: int | str) -> int | str:
    """Parse a value."""
    return value


class Client:
    def send(self) -> None:
        pass

    def receive(self) -> None:
        """Read the reply."""

    def _retry(self) -> None:
        pass

    def __init__(self) -> None:
        """:param: nothing to configure."""

    def __repr__(self) -> str:
        return "Client()"

    @property
    def size(self) -> int:
        return 0

    @size.setter
    def size(self, value: int) -> None:
        pass

    @override
    def close(self) -> None:
        pass


def _helper() -> None:
    pass


def internal() -> None:
    pass


__all__ = [
    "LIMIT",
    "NAME",
    "RETRIES",
    "TIMEOUT",
    "WINDOW",
    "Client",
    "fetch",
    "parse",
    "undocumented",
]
