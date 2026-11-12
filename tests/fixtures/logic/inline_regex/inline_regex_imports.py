from re import fullmatch
from typing import Callable


def is_word(text: str) -> bool:
    return fullmatch(r"\w+", text) is not None


def word_check(check: Callable[[str], bool]) -> bool:
    return check(fullmatch(r"\w+", "x") is not None)

