from dataclasses import dataclass
from unittest.mock import MagicMock


@dataclass
class Crate:
    size: int


class Shipper:
    def setUp(this) -> None:
        this.client = MagicMock()

    def ship(this) -> None:
        this.client.size = 2


@dataclass
class Gauge:
    level: int

    def reset(this) -> None:
        this.level = 0

    def reset_self(self) -> None:
        self.level = 0


@dataclass
class Dial:
    angle: int

    def turn(this) -> None:
        this.extra = 1
