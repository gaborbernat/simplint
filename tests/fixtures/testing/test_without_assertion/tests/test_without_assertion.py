import unittest
import warnings
from unittest import skip

import pytest

from .helpers import run_and_check


def compute() -> int:
    return 1


def test_nothing() -> None:
    compute()


def test_placeholder() -> None:
    pass


class TestComputation:
    runs = 0
    runs += 1

    def test_method(self) -> None:
        compute()

    def test_through_helper(self) -> None:
        self.verify()

    def test_self_helper_without_check(self) -> None:
        self.prepare()

    def test_inherited(self) -> None:
        self.inherited_helper()

    def verify(self) -> None:
        assert compute()

    def prepare(self) -> None:
        compute()


def test_assert() -> None:
    assert compute()


def test_raises() -> None:
    with pytest.raises(ValueError):
        compute()


def test_raise() -> None:
    if not compute():
        raise AssertionError


def test_check_helper() -> None:
    check_result(compute())


def test_unittest_style(case) -> None:
    case.assertEqual(compute(), 1)


def test_local_helper() -> None:
    local_helper()


def test_local_helper_without_check() -> None:
    noop_helper()


def test_imported_helper() -> None:
    run_and_check()


def test_callable_fixture(run) -> None:
    run(compute)


def test_matcher(pytester: pytest.Pytester) -> None:
    result = pytester.runpytest()
    result.stdout.fnmatch_lines(["*passed*"])


def test_warnings_escalated() -> None:
    with warnings.catch_warnings(action="error"):
        compute()


@pytest.mark.filterwarnings("error")
def test_marked_warnings_escalated() -> None:
    compute()


def test_simplefilter_strict() -> None:
    warnings.simplefilter("error")
    compute()


def test_filterwarnings_keyword() -> None:
    warnings.filterwarnings(action="error")
    compute()


def test_filterwarnings_ignore() -> None:
    warnings.filterwarnings("ignore")
    compute()


def test_optional_dependency() -> None:
    pytest.importorskip("numpy")
    compute()


class TestDerived(TestComputation):
    def test_method(self) -> None:
        super().test_method()


def test_recursive_helpers() -> None:
    ping(3)


def test_smoke() -> None:
    compute()


def test_documented() -> None:
    """The call must not crash."""
    compute()


def test_commented() -> None:
    # no error for an empty input
    compute()


@pytest.mark.skip(reason="not written yet")
def test_skipped() -> None:
    compute()


@skip("not written yet")
def test_skipped_by_name() -> None:
    compute()


@unittest.skip("not written yet")
def test_skipped_through_module() -> None:
    compute()


def test_benchmark(benchmark) -> None:
    benchmark(compute)


def test_helper_class() -> None:
    Case(compute)()


def test_chained_check() -> None:
    Data(compute).check()


class Case:
    def __init__(self, function) -> None:
        self.function = function

    def __call__(self) -> None:
        assert self.function()


class Data:
    def __init__(self, function) -> None:
        self.function = function


@pytest.fixture
def test_data() -> int:
    return compute()


def check_result(value: int) -> None:
    assert value


def local_helper() -> None:
    assert compute()


def noop_helper() -> None:
    compute()


def ping(count: int) -> None:
    if count:
        pong(count - 1)


def pong(count: int) -> None:
    ping(count)
