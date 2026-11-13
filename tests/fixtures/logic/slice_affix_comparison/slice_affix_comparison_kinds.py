import collections.abc
import getpass
import pathlib
import secrets
import shlex
import socket
import sys
import textwrap
import typing
from os import environ
from typing import Annotated, Any, final, override

import typing_extensions


class Base:
    def name(self) -> str:
        return "base"

    def plain(self) -> str:
        return "base"


class Box(Base):
    label: str

    @override
    def name(self) -> str:
        return "box"

    @typing_extensions.override
    def plain(self) -> str:
        return "box"

    def untyped(self):
        return "box"

    def ratio(self) -> float:
        return 1.0


@final
class Tag:
    title = "tag"


class Twice:
    def __init__(self) -> None:
        self.title: str = "a"
        self.title: str = "b"


LABEL = Box().label[:2] == "ab"
NAME = Box().name()[:2] == "ab"
PLAIN = Box().plain()[:2] == "ab"
UNTYPED = Box().untyped()[:2] == "ab"
TITLE = Tag().title[:2] == "ab"
TWICE = Twice().title[:2] == "ab"
RATIO = Box().ratio()[:2] == "ab"
PAIR = (Box() + Box())[:2] == "ab"


class Declared:
    def __init__(self) -> None:
        self.title: str


DECLARED = Declared().title[:2] == "ab"
MODULE_ATTRIBUTE = sys.version[:2] == "ab"
ENVIRON = environ.get("HOME", "ab")[:2] == "ab"


def library_calls(items):
    user = getpass.getuser()[:2] == "ab"
    hexed = secrets.token_hex()[:2] == "ab"
    url = secrets.token_urlsafe()[:2] == "ab"
    joined = shlex.join(items)[:2] == "ab"
    quoted = shlex.quote(items)[:2] == "ab"
    host = socket.gethostname()[:2] == "ab"
    filled = textwrap.fill(items)[:2] == "ab"
    indented = textwrap.indent(items, " ")[:2] == "ab"
    shortened = textwrap.shorten(items, 9)[:2] == "ab"
    home = pathlib.Path.home().name[:2] == "ab"
    return user, hexed, url, joined, quoted, host, filled, indented, shortened, home


def values(base, unknown):
    merged = {**base}
    mixed = {"a": unknown()}
    keyed = {unknown(): "a"}
    added = "a" + unknown()
    both = "a" + "b"
    return merged[:2] == "ab", mixed[:2] == "ab", keyed[:2] == "ab", added[:2] == "ab", both[:2] == "ab"


def annotations(
    empty: Annotated[()],
    nested: unknown.Generic[str],
    sequence: collections.abc.Sequence[str],
    anything: Any,
    module: typing.Sequence,
    typed: typing.Sequence[str],
):
    first = empty[:2] == "ab", nested[:2] == "ab", sequence[:2] == "ab"
    return first, anything[:2] == "ab", module[:2] == "ab", typed[:2] == "ab"


def declared_twice(flag):
    text: str = "a"
    if flag:
        text: str = "b"
    return text[:2] == "ab"


def cycle(flag):
    first = "a"
    second = first
    if flag:
        first = second
    return first[:2] == "ab"


def bindings(pair, items):
    [head, tail] = pair
    start, (inner, start) = pair
    try:
        pass
    except ValueError:
        pass

    def inner_function():
        nonlocal head
        head = "x"

    inner_function()
    words = (lambda: tail)()
    return words[:2] == "ab", any(item[:2] == "ab" for item in items), start, inner


def declares_nonlocal():
    shared = "a"

    def rebind():
        nonlocal shared
        shared = "b"
        return shared[:2] == "ab"

    return rebind()


VERSION = sys.version
STARTS_WITH = lambda: VERSION[:2] == "ab"  # noqa: E731
