__all__ = []

from dataclasses import dataclass


@dataclass
class Span:
    start: int
    end: int = 0

    def shift(self, cls: type) -> object:
        return cls(self.start, self.end)

    @classmethod
    def without_receiver() -> None:
        pass

    @classmethod
    def parse(cls, text: str) -> "Span":
        start, end = text.split(":")
        return cls(int(start), int(end))
