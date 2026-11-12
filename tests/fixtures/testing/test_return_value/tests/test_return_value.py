import unittest


def compute() -> int:
    return 1


def test_comparison() -> None:
    return compute() == 1


def test_boolean() -> None:
    return compute() and compute() > 0


def test_value() -> None:
    value = compute()
    return value


def test_conditional() -> None:
    if compute():
        return True
    assert False


class TestValues:
    def test_method(self) -> bool:
        return compute() != 2


def test_bare_return() -> None:
    if compute():
        return
    assert compute()


def test_none() -> None:
    return None


def test_delegates() -> None:
    return check_compute()


async def test_awaits() -> None:
    return await acheck()


def test_generator():
    yield compute()
    return compute() == 1


def test_nested() -> None:
    def inner() -> int:
        return compute()

    assert inner()


def test_suite() -> unittest.TestSuite:
    return unittest.TestSuite()


def check_compute() -> None:
    assert compute()


async def acheck() -> None:
    assert compute()


def helper() -> int:
    return compute()
