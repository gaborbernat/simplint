import re

_WORD = re.compile(r"\w+")


def is_word(text: str) -> bool:
    return _WORD.fullmatch(text) is not None
