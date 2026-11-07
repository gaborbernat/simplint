import pytest

from tests.shared_rows import SHARED_ARGS

_LOCAL = pytest.param("local-value", id="local")
_BARE = pytest.param("bare-value")
_PLAIN = ("plain-value", "x")
_REBOUND = pytest.param("first", id="first")
_REBOUND = pytest.param("second")


@pytest.mark.parametrize("value", [_LOCAL, _BARE, _PLAIN, _REBOUND, SHARED_ARGS])
def test_named_rows(value: str) -> None:
    assert value


@pytest.mark.parametrize("value", [*[_LOCAL, _BARE]])
def test_named_rows_in_spread(value: str) -> None:
    assert value
