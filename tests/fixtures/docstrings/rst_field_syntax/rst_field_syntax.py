def google(path: str) -> str:
    """Read a file.

    Args:
        path: Where it is.
    """
    return path


def numpy(path: str) -> str:
    """Read a file.

    Parameters
    ----------
    path : str
    """
    return path


def unknown(path: str) -> str:
    """Read a file.

    :parm path: Where it is.
    :param: Nothing named.
    :raises: Nothing named.
    :returns str: Named.
    """
    return path


def repeated(path: str) -> str:
    """Read a file.

    :param path: Where it is.
    :param path: Where it is, again.
    :returns: The text.
    """
    return path


def scattered(path: str, mode: str) -> str:
    """Read a file.

    :param path: Where it is.

    More prose after the fields.

    :param mode: How to open it.
    :raises OSError: When it is missing.
    :raises ValueError: When the mode is unknown.
    """
    return path + mode


def roles(path: str) -> str:
    """Read a file.

    :class:`str` is what comes back, and the example shows a field in a literal block::

        :param other: Not a field of this docstring.

    :param path: Where it is,
        across two lines.
    :returns: The text.
    """
    return path


def joined(path: str) -> str:
    """Read a file.
    :param path: Where it is.
    """
    return path


def twice(path: str) -> str:
    """Read a file.

    :param path: Where it is.
    :raises OSError: When it is missing.
    :raises OSError: When it cannot be read.
    """
    return path


def spelled(path: str, mode: str, flag: bool) -> str:
    """Read a file.

    :parameter path: Where it is.
    :arg mode: How to open it.
    :keyword flag: Whether to follow links.
    :raise OSError: When it is missing.
    :exception ValueError: When the mode is unknown.
    :return: The text.
    """
    return path + mode + str(flag)


def produced(count: int):
    """Count up.

    :argument count: How far.
    :key: Nothing named.
    :except: Nothing named.
    :yield: Each number.
    """
    yield from range(count)


class Settings:
    """Settings.

    :var name: The name.
    :ivar size: The size.
    :cvar limit: The limit.
    """

    limit = 1

    def __init__(self) -> None:
        self.name = ""
        self.size = 0


def unnamed(path: str) -> str:
    """Read a file.

    :param:
    """
    return path


def escaped(path: str) -> str:
    "Read a file.\n\n:arg path: Where it is.\n"
    return path
