import errors


def parse(text: str) -> int:
    """Parse a number.

    :param text: The text.
    :raises ValueError: When it is not a number.
    """
    if not text:
        raise ValueError("empty")
    if text == "-":
        raise errors.ParseError(text)
    if text == "?":
        raise LookupError
    if text == "!":
        raise LookupError
    try:
        return int(text)
    except TypeError as error:
        raise error
    except OverflowError:
        raise


def guarded(text: str) -> int:
    """Parse a number.

    :param text: The text.
    :raises errors.ParseError: When it is not a number.
    """
    try:
        if not text:
            raise KeyError(text)
    except KeyError:
        pass
    try:
        if text == "-":
            raise IndexError(text)
    except Exception:
        pass
    try:
        if text == "+":
            raise KeyError(text)
    except (KeyError, IndexError):
        pass
    if text == "*":
        raise errors.table[0]
    try:
        if text == "~":
            raise TimeoutError
    except:
        pass

    def nested() -> None:
        raise RuntimeError

    nested()
    if text == "?":
        raise errors.ParseError(text)
    raise NotImplementedError


def _private(text: str) -> int:
    """Parse a number."""
    raise ValueError(text)
