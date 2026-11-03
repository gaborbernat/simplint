def fetch(url: str, user_id: int, timeout: float) -> None:
    """Download a resource.

    :param url: The URL.
    :param user_id: The user ID.
    :param timeout: Seconds to wait for the first byte.
    """


class Session:
    """Talk to a server.

    :ivar host: The host value.
    :ivar port: Where it listens.
    """

    def __init__(self) -> None:
        self.host = "localhost"
        self.port = 80
