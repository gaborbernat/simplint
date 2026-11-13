import pytest


@pytest.fixture
def ufix_shared() -> int:
    return 1


@pytest.fixture
def ufix_orphan() -> int:
    return 2


@pytest.fixture(autouse=True)
def ufix_autouse() -> None:
    return None


@pytest.fixture
def anyio_backend() -> str:
    return "asyncio"
