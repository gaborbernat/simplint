from dataclasses import dataclass

__all__ = ["Release"]


@dataclass(frozen=True, slots=True)
class Release:
    name: str
    version: str


@dataclass(frozen=True, slots=True)
class Unlisted:
    value: int
