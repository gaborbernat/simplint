def empty(path: str) -> str:
    """Read a file.

    :param path:
    :returns:
    :raises OSError:
    """
    if not path:
        raise OSError
    return path


def aliased(path: str) -> str:
    """Read a file.

    :arg path:
    :return:
    """
    return path


def unnamed(path: str) -> str:
    """Read a file.

    :param:
    :returns str:
    """
    return path


def generated(count: int):
    """Count up.

    :yields:
    """
    yield from range(count)


def typed(path: str) -> str:
    """Read a file.

    :type path:
    :rtype:
    """
    return path


def described(path: str) -> str:
    """Read a file.

    :param path: Where it is.
    :raises OSError: When it is missing.
    :returns: The text.
    """
    return path


class Settings:
    """Settings.

    :var name:
    :ivar size:
    :cvar limit:
    """

    limit = 1

    def __init__(self) -> None:
        self.name = ""
        self.size = 0


def escaped(path: str) -> str:
    "Read a file.\n\n:param path:\n"
    return path
