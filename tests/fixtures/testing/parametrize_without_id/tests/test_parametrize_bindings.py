import os

import pytest

from tests.shared_rows import SHARED_ARGS

if os.name == "nt":
    BRANCH = pytest.param(1)
else:
    BRANCH = pytest.param(2)

TWICE = (1, 2)
TWICE = (3, 4)

for LOOPED in [pytest.param(5)]:
    pass

ONE = pytest.param(6)
WITH_ID = pytest.param(7, id="seven")
MIXED = pytest.param(8, id="eight")
MIXED = pytest.param(9)


@pytest.mark.parametrize("value", [BRANCH, TWICE, LOOPED, ONE, WITH_ID, MIXED, SHARED_ARGS])
def test_bindings(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [ONE, 7, 8])
def test_once_bound_again(value: int) -> None:
    assert value


for SEEN in range(3):
    pass

for LEFT, RIGHT in [(1, 2)]:
    pass


@pytest.mark.parametrize("value", [SEEN, LEFT, UNBOUND])
def test_unreadable_and_unbound(value: int) -> None:
    assert value


for TUPLED in (pytest.param(10),):
    pass


@pytest.mark.parametrize("value", [TUPLED])
def test_tuple_loop(value: int) -> None:
    assert value
