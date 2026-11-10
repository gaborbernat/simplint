from dataclasses import dataclass
from unittest import mock


@dataclass
class Prefs:
    retries: int = 0
    timeout: int = 1

    def update(self, **changes: object) -> None:
        for key, value in changes.items():
            setattr(self, key, value)


@dataclass
class Flags:
    verbose: bool = False


@dataclass
class Vault:
    depth: int = 1


@dataclass
class Dial:
    host: str = "localhost"


@dataclass
class Sensor:
    gain: int = 0


@dataclass
class Ledger:
    serial: str = ""

    def __post_init__(self) -> None:
        object.__setattr__(self, "serial", self.serial.upper())


@dataclass
class Gauge:
    reading: int = 0


@dataclass
class Control:
    size: int = 0


@dataclass
class _Result:
    status: str


def configure(key: str) -> Flags:
    flags = Flags()
    setattr(flags, key, True)
    return flags


def stub(monkeypatch: object) -> None:
    dial = Dial()
    monkeypatch.setattr(dial, "host", "example")
    sensor = Sensor()
    sensor.__setattr__("gain", 1)
    vault = Vault()
    with mock.patch.object(vault, "depth", 5):
        print(vault.depth)
    monkeypatch.setattr("pkg.Gauge.reading", "size")


def run() -> _Result:
    result = _Result(status="ok")
    result.log = "x"
    return result
