import pytest


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        pytest.param(1, 2, id="one"),
        pytest.param(2, 3),
        (3, [4]),
    ],
)
def test_values(value: int, expected: int) -> None:
    assert value + 1 == expected


CASES = [(1, [2])]


@pytest.mark.parametrize(("value", "expected"), CASES)
def test_from_variable(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.slow
@pytest.fixture()
@pytest.mark.parametrize("value")
def test_without_values(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [1, 2], ids=["one", "two"])
def test_with_ids(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [pytest.param(1, id=None)])
def test_none_id(value: int) -> None:
    assert value


@pytest.mark.parametrize("tag", ["select", "input", "a b", "x_1"])
def test_string_ids(tag: str) -> None:
    assert tag


@pytest.mark.parametrize("value", [1, 2.5, -1, True, None])
def test_scalar_ids(value: object) -> None:
    assert value


@pytest.mark.parametrize("tag", ["select", "input", "select"])
def test_repeated_string_ids(tag: str) -> None:
    assert tag


@pytest.mark.parametrize("value", [1, "1", 2])
def test_number_and_string_ids(value: object) -> None:
    assert value


@pytest.mark.parametrize("tag", ["café", "a\\b", b"bytes"])
def test_escaped_ids(tag: str) -> None:
    assert tag
