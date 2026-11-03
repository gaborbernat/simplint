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


def empty(path: str) -> str:
    """Read a file.

    :param path:
    :returns:
    :raises OSError:
    """
    if not path:
        raise OSError
    return path


def joined(path: str) -> str:
    """Read a file.
    :param path: Where it is.
    """
    return path
