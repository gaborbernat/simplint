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


def test_comprehension_elements(items: list[int]) -> None:
    assert [item if item else 0 for item in items] == items
    assert {item: 1 if item else 2 for item in items}
    assert {1 if item else 2 for item in items}
    assert sum(1 if item else 0 for item in items) == len(items)
    assert [item if (1 if item else 0) else 0 for item in items]
