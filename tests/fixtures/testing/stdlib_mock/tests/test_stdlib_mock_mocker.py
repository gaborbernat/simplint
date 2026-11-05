import unittest
from unittest import TestCase
from unittest.case import TestCase as Base

from pytest_mock import MockerFixture

from .unittest import mock


def test_client(mocker: MockerFixture) -> None:
    client = mocker.MagicMock()
    assert client and mock and unittest and TestCase and Base
