"""Lazy-capable imports next to imports the module needs while it loads."""

from __future__ import annotations

__lazy_modules__ = ["argparse", "zipfile"]

import argparse
import base64
import csv
import dataclasses
import decimal
import fractions
import hashlib
import json
import logging.config
import os
import pathlib
import shlex
import statistics
import string
import textwrap
import uuid
import zipfile
from typing import TYPE_CHECKING

from missing_lazy_module_pkg import helper

if TYPE_CHECKING:
    from collections.abc import Sequence

try:
    import tomllib
except ImportError:
    tomllib = None

ROOT = pathlib.Path(".")


def register(function):
    textwrap.dedent("x")
    return function


@register
def encode(value: bytes) -> bytes:
    return base64.b64encode(value)


def build_table() -> list[str]:
    return [string.ascii_letters]


class Reader:
    def __init__(self) -> None:
        self.dialect = csv.excel


@dataclasses.dataclass
class Record:
    key: uuid.UUID


@register
class Model:
    key: hashlib.sha256


TABLE = build_table()
READER = Reader()


def mean(values: Sequence[float]) -> float:
    return statistics.mean(values)


def ratio(value: fractions.Fraction) -> decimal.Decimal:
    return decimal.Decimal(value.numerator)


def quote(value: str) -> str:
    return shlex.quote(value)


shlex = None


def dump(value: object) -> str:
    return json.dumps(value) + os.sep + helper.VALUE + str(zipfile) + str(uuid.uuid4())


if __name__ == "__main__":
    print(argparse.ArgumentParser(), tomllib)
