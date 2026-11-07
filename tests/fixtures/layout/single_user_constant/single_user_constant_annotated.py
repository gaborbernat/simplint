from typing import Final

_PI_OVERRIDES: Final[dict[str, list[str]]] = {"<?namespace>": ["ProcessingInstruction"]}
_LIMIT: int = 30
_MODE: Final = "fast"
_CODES: dict[str, int] = {"ok": 0}


def lookup(text: str, expected: list[str]) -> list[str]:
    if text.endswith(">"):
        return _PI_OVERRIDES.get(text, expected)
    return expected


def connect() -> int:
    return _LIMIT


def mode() -> str:
    return _MODE


def merged() -> dict[str, int]:
    codes = {"error": 1}
    return {**_CODES, **codes}
