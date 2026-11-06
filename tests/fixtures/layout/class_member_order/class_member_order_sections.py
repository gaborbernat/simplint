from functools import cached_property
from typing import ClassVar, overload


class Layout:
    """Docstring."""

    def run(self) -> int:
        return self._step()

    def _step(self) -> int:
        return 1

    @property
    def size(self) -> int:
        return 1

    def __eq__(self, other: object) -> bool:
        return True

    @classmethod
    def from_json(cls, raw: str) -> "Layout":
        if raw:
            return cls()
        return cls

    def __repr__(self) -> str:
        return "Layout"

    @size.setter
    def size(self, value: int) -> None:
        pass

    def __init__(self) -> None:
        self.a = 1

    class Meta:
        pass

    LIMIT: ClassVar[int] = 3
    __slots__ = ("a",)

    def __init_subclass__(cls) -> None:
        pass

    def __post_init__(self) -> None:
        pass

    def __new__(cls) -> "Layout":
        return super().__new__(cls)


class Shapes:
    @staticmethod
    def unit() -> int:
        return 1

    @classmethod
    def build(cls) -> "Shapes":
        def helper() -> int:
            return 1

        return cls()

    @classmethod
    def nothing(cls) -> None:
        pass

    @classmethod
    def bare() -> int:
        return 1

    @cached_property
    def area(self) -> int:
        return 2

    def __str__(self) -> str:
        return "shape"

    def __init__(self) -> None:
        self.ready = True


class Totals:
    def run(self) -> int:
        return self._total()

    @cache
    def _total(self) -> int:
        return 1

    def __init__(self) -> None:
        self.value = 0


class Pinned:
    def load(self) -> int:
        return self.make()

    def make(self) -> int:
        return 1

    FACTORY = make

    def __init__(self) -> None:
        self.value = 0


class Tied:
    def load(self) -> int:
        return 1

    SCALE = 2

    def __init__(self, scale: int = SCALE) -> None:
        self.scale = scale


class Overloaded:
    def load(self, value: int | str) -> int:
        return 1

    @overload
    def read(self, value: int) -> int: ...

    @overload
    def read(self, value: str) -> str: ...

    def read(self, value: int | str) -> int | str:
        return value

    def __init__(self) -> None:
        self.value = 0


class Recursive:
    def even(self, n: int) -> bool:
        return True if n == 0 else self._odd(n - 1)

    def _odd(self, n: int) -> bool:
        return False if n == 0 else self.even(n - 1)

    def __init__(self) -> None:
        self.value = 0


class ConstructedTwice:
    def __init__(self) -> None:
        self.value = 0

    def __new__(cls) -> "ConstructedTwice":
        return super().__new__(cls)
