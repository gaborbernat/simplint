from typing import Any

type Blob = dict[str, Any]
type Payload = object


def first[T: Any](items: list[T]) -> T:
    return items[0]


def second[T = object](value: T) -> T:
    return value


class Box[V: object = Any]:
    pass


type Ok = int | str


def third[T: int = int](value: T) -> T:
    return value
