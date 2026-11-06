import itertools
from enum import Enum, auto

counter = itertools.count()


class AutoName(Enum):
    @staticmethod
    def _generate_next_value_(name: str, start: int, count: int, last_values: list[str]) -> str:
        return name

    NORTH = auto()

    def describe(self) -> str:
        return self.value

    def __str__(self) -> str:
        return self.name


class Counted:
    def method(self) -> int:
        return 1

    first = next(counter)

    def __repr__(self) -> str:
        return "Counted"


class Defaulted:
    def method(self, token: int = next(counter)) -> int:
        return token

    def __repr__(self) -> str:
        return "Defaulted"


class Plain:
    def method(self) -> int:
        return 1

    LIMIT = 3

    def __repr__(self) -> str:
        return "Plain"


class Deleted:
    def method(self) -> int:
        return 1

    def _scratch(self) -> int:
        return 2

    del _scratch

    def __init__(self) -> None:
        self.value = 0
