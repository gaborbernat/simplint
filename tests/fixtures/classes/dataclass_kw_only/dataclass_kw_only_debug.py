__all__ = []

from dataclasses import dataclass


@dataclass(frozen=True)
class Point:
    x: int
    y: int


def show() -> str:
    return f"{Point(1, 2)=}"
