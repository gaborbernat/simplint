import sys


def test_branches() -> None:
    if sys.platform == "win32":
        assert True is not False
    value = 1 if sys.maxsize else 2
    match value:
        case 1:
            pass
    assert [item for item in range(3) if item]


def helper() -> int:
    if sys.maxsize:
        return 1
    return 2


def test_nested() -> None:
    def inner() -> int:
        if sys.maxsize:
            return 1
        return 2

    assert inner()


def test_lambda() -> None:
    choose = lambda flag: 1 if flag else 2  # noqa: E731
    assert choose(True)
