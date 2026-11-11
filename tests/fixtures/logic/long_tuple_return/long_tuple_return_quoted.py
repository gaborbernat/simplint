def quoted(text: str) -> "tuple[str, ...]":
    return text, text, text, text


def bare(text: str) -> tuple[str, ...]:
    return text, text, text, text


def quoted_fixed(text: str) -> "tuple[str, str, str, str]":
    return text, text, text, text
