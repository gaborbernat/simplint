__lazy_modules__ = []

import base64
import csv
import decimal
import fractions
import json
import pickle
import shlex
import string
import uuid
import zlib


def helper() -> str:
    return string.ascii_letters


class Base:
    def __init_subclass__(cls) -> None:
        cls.quoted = shlex.quote("a")

    def unused(self) -> object:
        return zlib


class Meta(type):
    def __new__(mcs, name, bases, namespace):
        namespace["dialect"] = csv.excel
        return super().__new__(mcs, name, bases, namespace)


class Parent:
    def __init__(self) -> None:
        self.dumped = json.dumps(1)


class Child(Parent):
    pass


class Sub(Base):
    pass


class Model(metaclass=Meta):
    pass


class Later:
    def run(self) -> object:
        return pickle, uuid


A = (lambda: base64.b64encode(b"")) ()
C = sorted([1], key=lambda value: decimal.Decimal(value))
E = list(map(lambda _: helper(), [1]))
CHILD = Child()


def deferred() -> object:
    return (lambda: fractions.Fraction(1))()
