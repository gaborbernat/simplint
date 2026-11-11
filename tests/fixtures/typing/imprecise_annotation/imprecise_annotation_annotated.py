from typing import Annotated, Any


class Marker:
    def __init__(self, default: int) -> None:
        self.default = default


def limit(value: Annotated[int, Marker(default=len([object()]))]) -> int:
    return value


def pick(value: Annotated[int, object()]) -> int:
    return value


def meta(value: Annotated[int, {"type": Any}]) -> int:
    return value


def first(value: Annotated[object, "meta"]) -> int:
    return 1
