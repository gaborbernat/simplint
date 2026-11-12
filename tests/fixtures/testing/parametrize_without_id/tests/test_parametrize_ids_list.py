import pytest
import pytest as pt
from pytest import mark
from pytest.mark import parametrize

IDS = ["one", "two"]
NAMES = "value"
CASES = [(1, 2)]


@pytest.mark.parametrize(("value", "expected"), [(1, 2), (-1, 0)], ids=["positive", "negative"])
def test_pairs(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.parametrize("value", [1, 2], ids=("one", "two"))
def test_scalars(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", ids=["one", "two"], argvalues=[1, 2])
def test_keyword_order(value: int) -> None:
    assert value


@pytest.mark.parametrize(ids=["one", "two"], argnames="value", argvalues=[1, 2])
def test_keyword_first(value: int) -> None:
    assert value


@pytest.mark.parametrize(["value", "expected"], [(1, 2)], ids=["positive"])
def test_list_names(value: int, expected: int) -> None:
    assert value + 1 == expected


@pt.mark.parametrize("value", [1, 2], ids=["one", "two"])
def test_aliased_pytest(value: int) -> None:
    assert value


@mark.parametrize("value", [1, 2], ids=["one", "two"])
def test_imported_mark(value: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [(1, 2), (-1, 0)], ids=repr)
def test_callable_ids(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.parametrize("value", [1, 2], ids=IDS)
def test_named_ids(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [1, 2], ids=["one", None])
def test_none_entry(value: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [(1, 2), (-1, 0), (0, 1)], ids=["positive", "negative"])
def test_mismatched_counts(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.parametrize(("value", "expected"), [pytest.param(1, 2, id="positive"), (-1, 0)], ids=["positive", "negative"])
def test_mixed_rows(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.parametrize(
    ("value", "expected"),
    [
        (1, 2),
        # the negative case
        (-1, 0),
    ],
    ids=["positive", "negative"],
)
def test_comment_between_rows(value: int, expected: int) -> None:
    assert value + 1 == expected


CASE = (3, 4)


@pytest.mark.parametrize("value", [CASE], ids=["case"])
def test_name_row(value: tuple[int, int]) -> None:
    assert value


@pytest.mark.parametrize("value", [1, 2], None, ["one", "two"])
def test_positional_ids(value: int) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [(1, 2, 3)], ids=["three"])
def test_row_length(value: int, expected: int) -> None:
    assert value


@pytest.mark.parametrize("value", [-1, 2], ids=["negative", "two"])
def test_negative_row(value: int) -> None:
    assert value


@parametrize("value", [1], ids=["one"])
def test_bare_parametrize(value: int) -> None:
    assert value


@pytest.mark.parametrize(NAMES, [1], ids=["one"])
def test_named_argnames(value: int) -> None:
    assert value


@pytest.mark.parametrize(("value", NAMES), [(1, 2)], ids=["one"])
def test_mixed_argnames(value: int, expected: int) -> None:
    assert value + expected


@pytest.mark.parametrize(("value", "expected"), CASES, ids=["one"])
def test_named_rows(value: int, expected: int) -> None:
    assert value + 1 == expected


@pytest.mark.parametrize("value", [(1, 2)], ids=["pair"])
def test_tuple_value(value: tuple[int, int]) -> None:
    assert value
