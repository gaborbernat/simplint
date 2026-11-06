import pytest


@pytest.fixture
def ufix_base() -> int:
    return 3


@pytest.fixture
def ufix_top(ufix_base: int) -> int:
    return ufix_base


@pytest.fixture(name="ufix_named")
def _ufix_named_impl() -> int:
    return 4


@pytest.fixture
def ufix_unused() -> int:
    return 5


@pytest.mark.usefixtures("ufix_named")
def test_requests(ufix_top: int, ufix_shared: int) -> None:
    assert ufix_top == 3 and ufix_shared == 1
