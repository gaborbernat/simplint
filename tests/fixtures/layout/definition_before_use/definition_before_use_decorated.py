from collections.abc import Iterator
from contextlib import contextmanager
from dataclasses import dataclass

from api import router


class Item:
    name: str


@router.get("/item")
def read_item() -> Item:
    return Item()


class Payload:
    pass


@dataclass
class Envelope:
    payload: Payload

    def wrap(self) -> object:
        return Payload()


class Token:
    pass


def outer() -> object:
    def inner(token: Token) -> None:
        pass

    return inner


class Helper:
    pass


def caller() -> object:
    return Helper()


class Column:
    pass


@router.model
class Row:
    cell: Column

    def fill(self) -> object:
        return Column()


class Lease:
    pass


@contextmanager
def lease() -> Iterator[Lease]:
    yield Lease()


class Grant:
    pass


class Vault:
    @property
    def grant(self) -> Grant:
        return Grant()

    @grant.setter
    def grant(self, value: Grant) -> None:
        pass
