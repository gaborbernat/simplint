import unittest.mock
from unittest import mock
from unittest.mock import MagicMock, patch


def test_forms() -> None:
    assert unittest.mock and mock and MagicMock and patch


def test_nested() -> None:
    from unittest.mock import Mock

    assert Mock
