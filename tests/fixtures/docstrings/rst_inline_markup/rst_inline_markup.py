def open_file(path: str) -> None:
    """Open the file at :class:Path, as :py:func:open() does (:meth:Reader.read).

    See https://example.com:8080/path for details, and :term:`file` for the term.
    A literal ``open`` is fine, and ``this one is not.

    Example::

        call(``unbalanced)

    :param path: Where it is.
    """


def later_paragraph() -> None:
    """Stop.

    A ``closed`` literal and an ``open one
    that spans two lines``.

    The last ``one is open."""


def roles() -> None:
    """Roles with targets backticks cannot always fix.

    See :ref:getting-started, call :func:foo(x), and set :attr:debug=True.

    .. code-block:: python

        value = ``unclosed and :func:inside

    .. note::

        Prose with :func:outside here.

    >>> print(``doctest and :func:skipped)
    ``doctest and :func:skipped
    """
