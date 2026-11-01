import functools
import threading
from typing import Optional
from dataclasses import dataclass
from unittest import mock
from unittest.mock import MagicMock, create_autospec


@dataclass
class Record:
    uuid: str


@dataclass
class Result:
    log: str


@dataclass
class Extended:
    size: int


@dataclass
class Parent:
    name: str


class Child(Parent):
    @functools.cached_property
    def upper(self) -> str:
        return self.name.upper()


@dataclass
class Shape:
    corners: int


class Harness:
    def setUp(self) -> None:
        self.client = MagicMock()

    def test_client(self) -> None:
        self.client.uuid = "client"
        self.client.return_value.uuid = "called"


def test_mocks(mocker: object, patched: mock.Mock) -> None:
    plain = mock.Mock()
    plain.uuid = "plain"
    spec = create_autospec(Record)
    spec.uuid = "spec"
    fixture = mocker.MagicMock()
    fixture.uuid = "fixture"
    patched.uuid = "patched"
    with mock.patch.object(Record, "uuid") as replaced:
        replaced.uuid = "replaced"
    setattr(mocker.patch("module.record"), "uuid", "setattr")


def build(log: str) -> Result:
    res = Result(log="")
    res.log = log
    extended = Extended(size=1)
    extended.extra = True
    return res


def reshape(shape) -> None:
    shape.corners = 4


@dataclass
class Slotted:
    width: int
    __slots__ = ("width",)


@dataclass
class Linked:
    link: int

    def attach(self, other: "Linked") -> None:
        other.link = self.link


@dataclass
class Tagged:
    tag: str


class Runner:
    def setUp(self) -> None:
        self.stub = MagicMock()
        self.count = 0
        self.count += 1

    def tearDown(self) -> None:
        self.stub = Tagged(tag="")

    class Nested:
        def setUp(self) -> None:
            self.stub = MagicMock()

    def test_stub(self, maybe: None | Tagged, other: Optional[Tagged], rows: list[Tagged]) -> None:
        self.stub.tag = "stub"
        maybe.tag = "maybe"
        other.tag = "other"
        rows[0].tag = "row"


def rebind(resource: object) -> None:
    current = Tagged(tag="")
    current = MagicMock()
    current.tag = "rebound"
    with resource as handle, mock.patch("module.tag") as (first, second):
        handle.tag = "handle"


@dataclass
class Stratified:
    stratum: int


class ThreadStratified(Stratified, threading.local):
    pass
