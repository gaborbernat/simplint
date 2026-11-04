import builtins
import typing
from typing import Any, TypeIs
from typing import Any as Anything


def describe(value: object) -> str:
    return str(value)


def is_text(value: object) -> TypeIs[str]:
    return isinstance(value, str)


def guard(value: list[Any]) -> TypeIs[str]:
    return isinstance(value, str)


def takes_any(value: Any, *args: Any, **kwargs: Any) -> None:
    pass


def nested_param(value: dict[str, Any]) -> None:
    pass


def returns_any() -> Any:
    pass


def returns_list() -> list[Any]:
    pass


def aliased_param(value: Anything) -> None:
    pass


def qualified_param(value: typing.Any) -> None:
    pass


def string_param(value: "Any") -> None:
    pass


def string_return() -> "list[Any]":
    pass


class Point:
    def __eq__(self, other: object) -> bool:
        return other is self

    def __add__(self, other: dict[str, Any]) -> "Point":
        return self


def variables(data: list[int]) -> None:
    local: Any = data
    nested: list[Any] = []


CACHE: dict[str, object] = {}
VALUE: Any = 1
ALIASED: Anything = 2
QUALIFIED: typing.Any = 3
TEXT: "Any" = 4


def raw(value: builtins.object) -> None:
    pass
