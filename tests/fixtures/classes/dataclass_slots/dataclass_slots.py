import pickle
import threading
from dataclasses import dataclass
from functools import cached_property
from typing import ClassVar

import pytest


@dataclass
class Plain:
    width: int


@dataclass(frozen=False)
class Thawed:
    width: int


@dataclass(frozen=True, slots=True)
class Done:
    width: int


@dataclass(slots=False)
class Unslotted:
    width: int


@dataclass
class Empty:
    pass


@dataclass
class Declared:
    __slots__ = ("width",)
    width: int


@dataclass(frozen=True)
class Cached:
    radius: int

    @cached_property
    def doubled(self) -> int:
        return self.radius * 2


@dataclass(frozen=True)
class FileContext:
    path: str


class Context(FileContext, threading.local):
    pass


@dataclass
class Marked:
    version: str

    def mark(self) -> None:
        self.marked = True


@dataclass
class Loaded:
    version: str

    def load(self, values: dict[str, object]) -> None:
        self.__dict__.update(values)


@dataclass
class Tagged:
    name: str
    tag: ClassVar[str] = ""


def retag(tagged: Tagged) -> None:
    tagged.tag = "x"


@dataclass
class Pickled:
    version: str
    count: int = 0

    def bump(self) -> None:
        self.count += 1


def dump(pickled: Pickled) -> None:
    for protocol in range(2):
        pickle.dumps(pickled, protocol)


@dataclass(frozen=True)
class Stored:
    version: str


class CachedStored(Stored):
    def store(self) -> None:
        self.value = 1


class Base:
    pass


@dataclass
class Derived(Base):
    width: int


@dataclass
class UnhashableKey:
    text: str


def test_unhashable_key() -> None:
    with pytest.raises(TypeError):
        _ = {UnhashableKey("a"): True}
