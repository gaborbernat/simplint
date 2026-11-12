def after_bullet(path: str) -> str:
    """Read a file.

    - Where it is.
    :param path: Where it is.
    """
    return path


def after_literal(path: str) -> str:
    """Read a file.

    Example::

        call()
    :param path: Where it is.
    """
    return path


def after_text(path: str) -> str:
    """Read a file.

    Where it is.
    :param path: Where it is.
    :param other: Where it is, too.
    """
    return path


def in_quoted_literal(path: str) -> str:
    """Read a file.

    Example::

    :param path: Not a field here.
    """
    return path
