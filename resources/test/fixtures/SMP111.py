import dataclasses
import functools
from dataclasses import dataclass, field


@dataclass
class Plain:
    value: int


@dataclasses.dataclass(order=True)
class Ordered:
    value: int


@dataclass(frozen=True, slots=True, kw_only=True)
class Done:
    values: list[int] = field(default_factory=list)


@dataclass(frozen=False, slots=True, kw_only=True)
class Mutable:
    value: int


@functools.total_ordering
class Ordering:
    value: int
