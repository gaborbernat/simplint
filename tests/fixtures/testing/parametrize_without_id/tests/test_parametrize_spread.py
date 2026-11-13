import pytest

from tests.shared_rows import SHARED_ARGS

_REBOUND_ARGS = [(1, [2])]
_REBOUND_ARGS = [(3, [4])]
_NESTED_ARGS = [*_REBOUND_ARGS, (5, [6])]
_SPREAD_ARGS = [(7, [8])]


@pytest.mark.parametrize(("value", "expected"), [*[(13, [14])], *((15, [16]),), pytest.param(17, 18, id="s")])
def test_inline_spread(value: int, expected: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [*_REBOUND_ARGS])
def test_rebound_spread(value: int, expected: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [*SHARED_ARGS])
def test_imported_spread(value: int, expected: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [*_NESTED_ARGS])
def test_nested_spread(value: int, expected: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [*_SPREAD_ARGS])
def test_spread_once(value: int, expected: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [*_SPREAD_ARGS, (9, [10])])
def test_spread_twice(value: int, expected: int) -> None:
    assert value
