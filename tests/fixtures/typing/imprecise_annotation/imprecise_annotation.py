import builtins
import typing
from typing import Any, Optional, TypeIs, override, overload
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


class Base:
    def receive(self: Any) -> None:
        pass

    @classmethod
    def build(cls: Any) -> None:
        pass

    @override
    def run(self, value: Any) -> Any:
        return value

    @staticmethod
    def helper(value: Any) -> None:
        pass


@overload
def convert(value: int) -> int: ...
@overload
def convert(value: str) -> str: ...
def convert(value: Any) -> Any:
    return value


def maybe(value: Any | None) -> None:
    pass


def nullable(value: Optional[Any]) -> None:
    pass


def forward(*args: object, **kwargs: object) -> None:
    print(args, kwargs)


def forward_nested(*args: list[object]) -> None:
    print(args)
