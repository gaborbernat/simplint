import pytest


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        pytest.param(1, 2, id="one"),
        pytest.param(2, 3),
        (3, 4),
    ],
)
def test_values(value: int, expected: int) -> None:
    assert value + 1 == expected


CASES = [(1, 2)]


@pytest.mark.parametrize(("value", "expected"), CASES)
def test_from_variable(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.slow
@pytest.fixture()
@pytest.mark.parametrize("value")
def test_without_values(value: int) -> None:
    assert value
