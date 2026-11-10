import weakref
from dataclasses import dataclass
from unittest import mock
from weakref import proxy


def register(cls: type) -> type:
    return cls


@dataclass
class Beacon:
    label: str


@dataclass
class Tether:
    label: str


@dataclass
class Gate:
    host: str

    def send(self) -> str:
        return self.host


@dataclass
@register
class Lantern:
    name: str


@register
@dataclass
class Anchor:
    name: str


@dataclass
class _Result:
    status: str


def watch() -> None:
    beacon = Beacon(label="b")
    reference = weakref.ref(beacon)
    tether = Tether(label="t")
    shadow = proxy(tether)
    print(reference(), shadow)


def patched() -> None:
    gate = Gate(host="localhost")
    with mock.patch.object(gate, "send", return_value="stubbed"):
        print(gate.send())


def run() -> _Result:
    result = _Result(status="ok")
    result.log = "x"
    return result
