import json
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


def load(path: str) -> str | None:
    """Load a file.

    :param path: The file.
    :returns: Its text.
    """
    try:
        if not path:
            raise FileNotFoundError(path)
    except OSError:
        return None
    try:
        raise KeyError(path)
    except LookupError:
        return None


def hierarchy() -> None:
    """Raise and catch each built-in exception through a base."""
    try:
        raise BrokenPipeError
    except ConnectionError:
        pass
    try:
        raise ConnectionError
    except OSError:
        pass
    try:
        raise ZeroDivisionError
    except ArithmeticError:
        pass
    try:
        raise UnicodeDecodeError
    except UnicodeError:
        pass
    try:
        raise UnicodeError
    except ValueError:
        pass
    try:
        raise ModuleNotFoundError
    except ImportError:
        pass
    try:
        raise RecursionError
    except RuntimeError:
        pass
    try:
        raise UnboundLocalError
    except NameError:
        pass
    try:
        raise TabError
    except IndentationError:
        pass
    try:
        raise IndentationError
    except SyntaxError:
        pass
    try:
        raise ValueError
    except KeyError:
        pass


def documented_base(path: str) -> None:
    """Open a file.

    :param path: The file.
    :raises OSError: When it cannot be opened.
    """
    raise FileNotFoundError(path)


def exits() -> None:
    """Stop."""
    try:
        raise SystemExit(1)
    except Exception:
        pass


def documented_warning_base() -> None:
    """Warn.

    :raises Warning: Always.
    """
    raise UserWarning


def documented_standard_base(text: str) -> None:
    """Decode a document.

    :param text: The document.
    :raises ValueError: When it is not JSON.
    """
    raise json.JSONDecodeError(text, text, 0)


def documented_alias(path: str) -> None:
    """Open a file.

    :param path: The file.
    :raises IOError: When it cannot be opened.
    """
    raise FileNotFoundError(path)


class LocalError(LookupError):
    """A local failure."""


def documented_local_base() -> None:
    """Look up.

    :raises LookupError: Always.
    """
    raise LocalError


def caught_by_standard_base(path: str) -> None:
    """Open a file.

    :param path: The file.
    """
    try:
        raise json.JSONDecodeError(path, path, 0)
    except ValueError:
        pass


def undocumented_warning() -> None:
    """Warn."""
    raise UserWarning


def caught_by_other(text: str) -> None:
    """Parse a number.

    :param text: The text.
    """
    try:
        raise errors.ParseError(text)
    except KeyError:
        pass


def caught_broadly(text: str) -> None:
    """Parse a number.

    :param text: The text.
    """
    try:
        raise errors.ParseError(text)
    except Exception:
        pass
