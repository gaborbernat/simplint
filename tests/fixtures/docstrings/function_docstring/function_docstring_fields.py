def _run(args: list[str] | None = None) -> int:
    """
    Run the formatter.

    :param args: command line arguments, by default use sys.argv[1:]
    :return: exit code - 0 means already formatted correctly, otherwise 1
    """
    return 0


def _fields_only(name: str) -> None:
    """
    :param name: the key to store under
    """
    print(name)


def _google(name: str) -> str:
    """Store the entry under a name.

    Args:
        name: the key the entry is stored under

    Returns:
        the stored entry
    """
    return name


def _numpy(name: str) -> str:
    """Store the entry under a name.

    Parameters
    ----------
    name
        the key the entry is stored under

    Returns
    -------
    str
        the stored entry
    """
    return name


def _prose_after(name: str) -> None:
    """Store the entry under a name.

    :param name: the key the entry is stored under
    :raises KeyError: when the name is taken

    The first entry wins on a clash.
    """
    print(name)


def _role(name: str) -> None:
    """Store the entry under a name.

    :py:func:`store` does the work, see `:param:` for the details.
    """
    print(name)


def _sections_only(name: str) -> None:
    """
    Args:
        name: the key the entry is stored under
    """
    print(name)
