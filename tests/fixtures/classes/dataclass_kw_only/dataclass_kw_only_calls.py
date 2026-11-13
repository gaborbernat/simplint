__all__ = []

import functools
from dataclasses import KW_ONLY, InitVar, dataclass, field
from typing import ClassVar, Generic, TypeVar

from dc_bases import ImportedBase, LaterBase

Item = TypeVar("Item")


@dataclass
class Endpoint:
    host: str
    port: int = 80

    @classmethod
    def parse(cls, text: str) -> "Endpoint":
        host, port = text.split(":")
        return cls(host, int(port))


@dataclass
class Layered(Generic[Item]):
    total: ClassVar[int] = 0
    seed: InitVar[int]
    cache: dict[str, int] = field(init=False, default_factory=dict)
    label: str = field(kw_only=True, default="")
    value: Item | None = None

    def __post_init__(self, seed: int) -> None:
        self.cache["seed"] = seed


@dataclass
class Rooted:
    root: str


@dataclass
class Grown(Rooted):
    leaf: str
    _: KW_ONLY
    weight: int = 0


@dataclass
class Spread:
    first: int
    second: int


@dataclass
class Handed:
    value: int


@dataclass
class Matched:
    left: int
    right: int


@dataclass
class Constructed:
    value: int

    def __init__(self, value: int) -> None:
        self.value = value


@dataclass
class Cloned:
    value: int

    def copy(self) -> "Cloned":
        return type(self)(self.value)


@dataclass
class Overfed:
    value: int


@dataclass
class Typed:
    value: int
    note: "str" = ""


def build(pairs: list[tuple[int, int]], match_me: object) -> object:
    endpoint = Endpoint("localhost", (8080))
    keyed = Endpoint(**{"host": "localhost"})
    mixed = Endpoint("localhost", **{"port": 80})
    layered = Layered(1, label="x")
    grown = Grown("root", "leaf", weight=2)
    spread = Spread(*pairs[0])
    handed = functools.partial(Handed, 1)
    match match_me:
        case Matched(1, 2):
            pass
    overfed = Overfed(1, 2)
    typed: Typed = Typed(sum(value for value in (1, 2)))
    single = Typed(value for value in (1,))
    if isinstance(match_me, (Typed, Endpoint)) and issubclass(type(match_me), Typed):
        return Typed.__name__
    return endpoint, keyed, mixed, layered, grown, spread, handed, overfed, typed, single


@dataclass
class Quoted:
    total: "ClassVar[int]" = 0


@dataclass
class Spliced:
    value: int = field(**{"default": 0})


@dataclass
class Foreign(functools.partial):
    value: int


class Registry(metaclass=type):
    pass


@dataclass
class FromImported(ImportedBase):
    value: int


@dataclass
class FromLater(LaterBase):
    value: int


class LaterBase:
    pass
