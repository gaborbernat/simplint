from typing import TypeIs


def describe(value: object) -> str:
    return str(value)


def is_text(value: object) -> TypeIs[str]:
    return isinstance(value, str)


class Point:
    def __eq__(self, other: object) -> bool:
        return other is self


CACHE: dict[str, object] = {}
