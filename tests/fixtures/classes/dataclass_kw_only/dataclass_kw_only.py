__all__ = []

import dataclasses
from dataclasses import dataclass


@dataclass
class Plain:
    value: int


@dataclasses.dataclass(frozen=True, slots=True)
class Options:
    value: int


@dataclass(kw_only=False)
class OptedOut:
    value: int


@dataclass(kw_only=True)
class Done:
    value: int


@dataclass(init=False)
class _HandWritten:
    code: int

    def __init__(self, code: int) -> None:
        self.code = code


@dataclass(init=True)
class _Generated:
    code: int
