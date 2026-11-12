from enum import Enum, StrEnum


class Shade(Enum):
    LIGHT = 1
    DARK = 2


class Hue(StrEnum):
    RED = "red"
    BLUE = "blue"


class Palette:
    def paint(self, address: str) -> None:
        print(address)
