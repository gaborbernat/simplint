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


def raised_by_helper_twice(first: str, second: str) -> str:
    """Check two names.

    :param first: The first.
    :param second: The second.
    :returns: The names.
    :raises KeyError: When one is unknown.
    """
    return _unraised_validated(first) + _unraised_validated(second)


def raised_in_with(lock: object, path: str) -> str:
    """Return a path under a lock.

    :param lock: The lock.
    :param path: The path.
    :returns: The path.
    :raises KeyError: When the lock's exit says so.
    """
    with lock:
        return path


def raised_by_assert(text: str) -> str:
    """Check a name.

    :param text: The text.
    :returns: The text.
    :raises AssertionError: When it is empty.
    """
    assert text
    return text


def raised_bare() -> None:
    """Re-raise.

    :raises KeyError: When a handler around the call caught one.
    """
    raise


def raised_with_cause(text: str) -> None:
    """Reject a name.

    :param text: The text.
    :raises ValueError: Always.
    :raises RuntimeError: Never.
    """
    raise ValueError(text) from None


def raised_in_set(names: list[str]) -> set[str]:
    """Collect names.

    :param names: The names.
    :returns: The names.
    :raises KeyError: When the iterable says so.
    """
    return {name for name in names}


def raised_in_dict(names: list[str]) -> dict[str, int]:
    """Count names.

    :param names: The names.
    :returns: The counts.
    :raises KeyError: When the iterable says so.
    """
    return {name: 1 for name in names}


def unraised_over_display() -> dict[str, int]:
    """Count names.

    :returns: The counts.
    :raises KeyError: Never.
    """
    return {name: 1 for name in ("a", "b")}


def unraised_subscript_other(names: dict[str, str]) -> str:
    """Return the first name.

    :param names: The names.
    :returns: The name.
    :raises RuntimeError: Never.
    """
    return names["first"]


def unraised_arithmetic(total: int, count: int) -> float:
    """Average the total.

    :param total: The total.
    :param count: The count.
    :returns: The average.
    :raises RuntimeError: Never.
    """
    return total / count


def unraised_beside_nested(text: str) -> str:
    """Wrap a name.

    :param text: The text.
    :returns: The text.
    :raises RuntimeError: Never.
    """

    def inner() -> str:
        return text

    return text


def raised_by_division(total: int, count: int) -> float:
    """Average the total.

    :param total: The total.
    :param count: The count.
    :returns: The average.
    :raises ZeroDivisionError: When the count is zero.
    """
    return total / count


def unraised_over_literal(text: str) -> str:
    """Join the parts.

    :param text: The text.
    :returns: The text.
    :raises RuntimeError: Never.
    """
    for part in ("a", "b"):
        text += part
    return text
