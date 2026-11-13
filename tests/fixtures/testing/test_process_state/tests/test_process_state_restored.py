import os
import sys
import unittest
from unittest import mock

import pytest


@mock.patch.dict(os.environ)
def test_decorated_patch_dict() -> None:
    os.environ["MODE"] = "debug"


def test_mocker_patch_dict(mocker) -> None:
    mocker.patch.dict(os.environ, {"MODE": "x"})
    os.environ["MODE"] = "debug"


def test_monkeypatch_first(monkeypatch) -> None:
    monkeypatch.delenv("MODE", raising=False)
    os.environ["MODE"] = "debug"
    monkeypatch.syspath_prepend("/tmp")
    sys.path.append("/opt")
    monkeypatch.chdir("/")
    os.chdir("/srv")


def test_patch_dict_of_unknown(mocker) -> None:
    mocker.patch.dict(SETTINGS)
    mocker.patch.dict()
    os.environ["MODE"] = "debug"


@mock.patch.dict(os.environ)
class TestPatchedClass(unittest.TestCase):
    def test_write(self) -> None:
        os.environ["MODE"] = "debug"


class Base(unittest.TestCase):
    pass


class TestInherits(Base):
    def test_write(self) -> None:
        os.environ["MODE"] = "debug"


class TestPlain(unittest.TestCase):
    def test_write(self) -> None:
        os.environ["MODE"] = "debug"


class TestObject(object):
    def test_write(self) -> None:
        os.environ["MODE"] = "debug"
