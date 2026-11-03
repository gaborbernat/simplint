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
