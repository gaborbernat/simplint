import dataclasses
import functools
from dataclasses import dataclass, field


@dataclass
class Plain:
    width: int


@dataclasses.dataclass(order=True, kw_only=True)
class Ordered:
    rank: int


@dataclass(frozen=True, slots=True)
class Done:
    values: list[int] = field(default_factory=list)


@dataclass(frozen=False, slots=True)
class OptedOut:
    height: int


@dataclass
class Base:
    depth: int


@dataclass
class Child(Base):
    extra: int


@dataclass
class Counter:
    count: int


@dataclass
class Labelled:
    label: str


@dataclass
class Cached:
    value: int

    @functools.cached_property
    def doubled(self) -> int:
        return self.value * 2


@dataclass
class Initialized:
    value: int

    def __post_init__(self) -> None:
        self.total = self.value


@dataclass
class Guarded:
    size: int

    def __delattr__(self, name: str) -> None:
        raise AttributeError(name)


@functools.total_ordering
class Ordering:
    value: int


def bump(counter: Counter, labelled: Labelled) -> None:
    counter.count += 1
    setattr(labelled, "label", "new")
