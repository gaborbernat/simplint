import pytest


@pytest.mark.parametrize("kind", [*(pytest.param(kind, id=kind) for kind in ("a", "b")), pytest.param("c", id="c")])
def test_kind(kind: str) -> None:
    assert kind


@pytest.mark.parametrize("case", [*(pytest.param(*c, id=c[0]) for c in (("a", 1), ("b", 2)))])
def test_case(case: str) -> None:
    assert case


_SIMPLE_ARGS = [pytest.param("a", "b", id="ab")]
_UNIDENTIFIED_ARGS = [(1, [2]), pytest.param(3, 4, id="three")]


@pytest.mark.parametrize(("value", "expected"), [*_SIMPLE_ARGS, pytest.param("c", "d", id="cd")])
def test_named_spread(value: str, expected: str) -> None:
    assert value


@pytest.mark.parametrize(("value", "expected"), [*_UNIDENTIFIED_ARGS])
def test_named_spread_without_id(value: int, expected: int) -> None:
    assert value
