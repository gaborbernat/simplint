import pytest
import pytest as pt
from pytest import mark
from pytest.mark import parametrize
from shared_ids import shared_ids

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


@pytest.mark.parametrize(("value",), [(1,), (2,)], ids=["one", "two"])
def test_one_name_tuple(value: int) -> None:
    assert value


@pytest.mark.parametrize(["value"], [[1], [2]], ids=["one", "two"])
def test_one_name_list(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [(1,), (2,)], ids=["one", "two"])
def test_one_name_string_row(value: tuple[int]) -> None:
    assert value


@pytest.mark.parametrize("value,", [(1,), (2,)], ids=["one", "two"])
def test_one_name_trailing_comma(value: int) -> None:
    assert value


@pytest.mark.parametrize("value, expected", [(1, 2), (3, 4)], ids=["one", "three"])
def test_string_names(value: int, expected: int) -> None:
    assert value + expected


@pytest.mark.parametrize((), [()], ids=["empty"])
def test_no_names() -> None:
    assert True


REBOUND_IDS = ["one", "two"]
REBOUND_IDS = ["first", "second"]
NUMBER_IDS = [1, 2]


@pytest.mark.parametrize("value", [1, 2], ids=REBOUND_IDS)
def test_rebound_ids(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [1, 2], ids=NUMBER_IDS)
def test_number_ids(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [1, 2], ids=shared_ids)
def test_imported_ids(value: int) -> None:
    assert value


def test_local_ids() -> None:
    local_ids = ["one", "two"]

    @pytest.mark.parametrize("value", [1, 2], ids=local_ids)
    def test_local(value: int) -> None:
        assert value
