def fetch(url: str, _retries: int = 3) -> None:
    """Download a resource.

    :param url: Address of the resource.
    :param _retries: How often to retry, for the test suite.
    """


class StoreCache:
    """Cache values.

    :ivar size: How many it holds.
    :ivar _store: The internal storage.
    """

    def __init__(self) -> None:
        self.size = 0
        self._store = {}


def _helper(_value: int) -> None:
    """Help.

    :param _value: Internal.
    """


def unwrap(__obj: object) -> None:
    """Unwrap a callable.

    :param __obj: The positional-only callable.
    """


class Wrapper:
    """Wrap a callable.

    :ivar __wrapped__: The wrapped callable.
    """

    def __init__(self, size: int, _cache: bool = False) -> None:
        """Make the wrapper.

        :param size: How many it holds.
        :param _cache: Whether to cache, for the test suite.
        """
        self.__wrapped__ = size
