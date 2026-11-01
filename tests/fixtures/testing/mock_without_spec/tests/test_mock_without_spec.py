import requests
from httpx import Client
from unittest import mock
from unittest.mock import MagicMock, patch

import pytest_mock


def test_mocks(mocker: pytest_mock.MockerFixture) -> None:
    bare = MagicMock()
    specced = MagicMock(spec=int)
    positional = mock.Mock(int)
    fixture = mocker.MagicMock()
    with patch("os.getcwd"):
        pass
    with patch("os.getcwd", autospec=True):
        pass
    mocker.patch.object(mock, "Mock")
    mocker.patch.object(mock, "Mock", autospec=True)
    assert bare and specced and positional and fixture


def test_plain() -> None:
    assert str(1)


def test_http(client: Client) -> None:
    client.patch("/items", json={})
    requests.patch("https://example.com")


def test_disabled_spec() -> None:
    off = MagicMock(spec=None)
    with patch("os.getcwd", autospec=False):
        assert off


def test_replacement(mocker: pytest_mock.MockerFixture) -> None:
    mocker.patch("pkg.FLAG", new=False)
    mocker.patch("pkg.HANDLER", new=None)
