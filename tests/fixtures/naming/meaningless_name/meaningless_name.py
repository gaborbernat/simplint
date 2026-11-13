from typing import override

foo = 1
for baz in range(3):
    print(baz)

with open("file") as qux:
    print(qux)

if (quux := 3) > 2:
    print(quux)


def scale(factor: float, quux: float) -> float:
    return factor * quux


callback = lambda foo: foo  # noqa: E731


def repeat(foo: int) -> int:
    foo = foo + 1
    return foo


def fails() -> None:
    try:
        run()
    except OSError as qux:
        print(qux)


def captures(point: object) -> None:
    match point:
        case {"a": 1, **baz}:
            print(baz)
        case quux:
            print(quux)


class Cache:
    def __init__(self) -> None:
        self.foo = {}
        self.name = "cache"
        self.foo = {}
        other.quux = 3

    @classmethod
    def build(cls) -> None:
        cls.foo = 1

    @staticmethod
    def helper(foo: int) -> int:
        return foo

    def nested(self) -> None:
        def inner() -> None:
            self.foo = 2

        inner()


class Base:
    def run(self, quux: int) -> None:
        pass


class Child(Base):
    @override
    def run(self, quux: int) -> None:
        pass


def names() -> None:
    foo_count = 1
    FOO = 2
    _baz = 3
    Foo = type("Foo", (), {})
    bar = data = 4
    print(foo_count, FOO, _baz, Foo, bar, data)


from dataclasses import dataclass  # noqa: E402
from typing import TypedDict  # noqa: E402

from pydantic import BaseModel  # noqa: E402


@dataclass
class Payload:
    foo: str


class Model(BaseModel):
    bar: int


class Row(TypedDict):
    baz: str


class Plain:
    foo: str
