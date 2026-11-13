import json


def unraised_stale(text: str) -> str:
    """Name a greeting.

    :param text: The text.
    :returns: The greeting.
    :raises ValueError: When the text is empty.
    """
    return text + "!"


def unraised_through_helper(text: str) -> str:
    """Check a name.

    :param text: The text.
    :returns: The text.
    :raises ValueError: When it is empty.
    :raises KeyError: When it is unknown.
    """
    return _unraised_validated(text)


def _unraised_validated(text: str) -> str:
    if not text:
        raise ValueError(text)
    return text


def unraised_base(path: str) -> str:
    """Open a file.

    :param path: The path.
    :returns: The path.
    :raises OSError: When it is missing.
    """
    raise FileNotFoundError(path)


def unraised_reraised(text: str) -> str:
    """Parse a name.

    :param text: The text.
    :returns: The name.
    :raises ValueError: When the text is empty.
    """
    try:
        return _unraised_validated(text)
    except ValueError:
        raise


def unraised_called_elsewhere(text: str) -> object:
    """Load a document.

    :param text: The text.
    :returns: The document.
    :raises ValueError: When the text is not JSON.
    """
    return json.loads(text)


def unraised_subscript(names: dict[str, str]) -> str:
    """Return the first name.

    :param names: The names.
    :returns: The name.
    :raises KeyError: When there is none.
    :raises Exception: When anything fails.
    """
    return names["first"]


class Unraised:
    """A holder."""

    def check(self, text: str) -> str:
        """Check a name.

        :param text: The text.
        :returns: The text.
        :raises RuntimeError: When it is unknown.
        """
        return self._known(text)

    def _known(self, text: str) -> str:
        if not text:
            raise ValueError(text)
        return text


def raised_warning_base() -> None:
    """Warn.

    :raises Warning: Always.
    """
    raise UserWarning


def raised_standard_base(text: str) -> None:
    """Decode a document.

    :param text: The document.
    :raises ValueError: When it is not JSON.
    """
    raise json.JSONDecodeError(text, text, 0)


def raised_alias(path: str) -> None:
    """Open a file.

    :param path: The file.
    :raises IOError: When it cannot be opened.
    :raises socket.error: When it cannot be opened.
    """
    raise FileNotFoundError(path)


def unraised_sibling() -> None:
    """Warn.

    :raises DeprecationWarning: Never.
    """
    raise UserWarning
