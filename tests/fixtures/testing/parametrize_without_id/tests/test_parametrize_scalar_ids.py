import pytest


@pytest.mark.parametrize("value", [0x10, 16])
def test_hex_and_decimal(value: int) -> None:
    assert value


@pytest.mark.parametrize("value", [1e3, 1000.0])
def test_exponent_and_float(value: float) -> None:
    assert value


@pytest.mark.parametrize("value", [0x10, 17, 1e3])
def test_distinct_numbers(value: float) -> None:
    assert value


@pytest.mark.parametrize("value", [1e-5, 0.00001, 1e16, 1e+16, 0.5, 0.05, -0.0, 0.0, 1e400, 1e400])
def test_float_reprs(value: float) -> None:
    assert value


@pytest.mark.parametrize("value", [-0, 0, 1j, 2j, 100000000000000000000, 100000000000000000000, 0x1_0000_0000_0000_0000])
def test_integer_and_complex_ids(value: complex) -> None:
    assert value


@pytest.mark.parametrize(("raw", "expected"), [("FALSE", "false"), ("True", "true"), (1, None)])
def test_tuple_rows_of_scalars(raw: object, expected: object) -> None:
    assert raw != expected


@pytest.mark.parametrize(("raw", "expected"), [("a", "b"), ("a", "b"), ("a", object()), ("a",)])
def test_tuple_rows_pytest_cannot_name(raw: object, expected: object) -> None:
    assert raw != expected


@pytest.mark.parametrize("header", [b"gzip", b"br", b"a\\b"])
def test_bytes_rows(header: bytes) -> None:
    assert header


@pytest.mark.parametrize(("raw", "expected"), [["FALSE", "false"], ["True", "true"]])
def test_list_rows_of_scalars(raw: str, expected: str) -> None:
    assert raw != expected
