import typing
from typing import Self


class Date:
    def __init__(self, ordinal: int) -> None:
        self.ordinal = ordinal

    def describe(self) -> str:
        return str(self.ordinal)

    @classmethod
    def fromordinal(cls, ordinal: int):
        return cls(ordinal)

    @classmethod
    def now(cls):
        return cls.today()

    @classmethod
    def today(cls):
        return cls.fromordinal(1)

    @classmethod
    def ping(cls):
        return cls.pong()

    @classmethod
    def pong(cls):
        return cls.ping()

    @classmethod
    def count(cls):
        return cls.total()

    @classmethod
    def total(cls) -> int:
        return 1

    @classmethod
    def parse(cls, text: str) -> Self:
        return cls.registry[text]

    @classmethod
    def load(cls, text: str) -> typing.Self:
        return cls.registry[text]

    @classmethod
    def quoted(cls, text: str) -> "Date":
        return cls.registry[text]

    @classmethod
    def named(cls, text: str) -> Date:
        return cls.registry[text]

    registry: typing.ClassVar[dict[str, "Date"]] = {}


class Stamp:
    def __init__(self, value: int) -> None:
        self.value = value

    def describe(self) -> str:
        return str(self.value)

    @classmethod
    def _build(cls, value: int):
        result = cls(value)
        if value < 0:
            return result
        return result

    @classmethod
    def at(cls, value: int):
        return cls._build(value)

    @classmethod
    def copied(cls, value: int):
        stamp = cls.at(value)
        return stamp

    @classmethod
    def rebound(cls, value: int):
        result = cls(value)
        result = str(result)
        return result

    @classmethod
    def conditional(cls, value: int):
        if value:
            result = cls(value)
            return result
        return None


class Moment:
    def __init__(self, value: int) -> None:
        self.value = value

    def describe(self) -> str:
        return str(self.value)

    @classmethod
    def _fromstamp(cls, value: int, shift: bool):
        result = cls(value)
        if shift:
            result = result.shifted()
        return result

    @classmethod
    def fromstamp(cls, value: int):
        return cls._fromstamp(value, False)

    @classmethod
    def fallback(cls, value: int):
        result = None
        if value:
            result = cls(value)
        return result

    def shifted(self) -> "Moment":
        return self
