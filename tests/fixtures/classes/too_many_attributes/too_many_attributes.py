import dataclasses
from dataclasses import KW_ONLY, InitVar, dataclass
from typing import ClassVar


class Wide:
    def __init__(self) -> None:
        self.a = self.b = self.c = self.d = self.e = self.f = 0
        self.g = self.h = self.i = self.j = self.k = 0

    def later(self) -> None:
        self.l = 1
        self.m = 2
        self.a = 3


class AtLimit:
    def __init__(self) -> None:
        self.a = self.b = self.c = self.d = self.e = self.f = 0
        self.g = self.h = self.i = self.j = self.k = self.l = 0

    @classmethod
    def build(cls) -> None:
        cls.m = 1


@dataclass
class Record:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: int
    g: int
    h: int
    i: int
    j: int
    _: KW_ONLY
    k: int
    l: int
    m: int
    registry: ClassVar[list[str]] = []


@dataclasses.dataclass(frozen=True)
class Mixed:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: int
    g: int
    h: int
    i: int
    j: int
    k: int

    def __post_init__(self) -> None:
        object.__setattr__(self, "l", 0)
        self.extra = self.more = 0  # type: ignore[misc]


class NotADataclass:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: int
    g: int
    h: int
    i: int
    j: int
    k: int
    l: int
    m: int


@dataclass
class AtLimitWithMarkers:
    a: int
    b: int
    c: int
    d: int
    e: int
    f: int
    g: int
    h: int
    i: int
    j: int
    k: int
    l: int
    seed: InitVar[int]
    registry: "ClassVar[list[str]]" = []
