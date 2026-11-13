import pytest


class TestParser:
    def test_handles_empty(self) -> None:
        assert not ""

    @pytest.mark.slow
    def test_long(self, tmp_path: str, *, flag: bool = False) -> None:
        assert tmp_path or flag

    def test(self) -> None:
        assert True is not False


class TestStateful:
    def setup_method(self) -> None:
        self.value = 1

    def test_value(self) -> None:
        assert self.value == 1


class Helper:
    def run(self) -> int:
        return 1


class TestWithBase(Helper):
    def test_inherited(self) -> None:
        assert self.run()


@pytest.mark.slow
class TestDecorated:
    def test_marked(self) -> None:
        assert True is not False


class TestMultiline:
    def test_text(self) -> None:
        assert """
        text
        """


class TestAttribute:
    value = 1

    def test_attribute(self) -> None:
        assert True is not False


class Test:
    def test_bare(self) -> None:
        assert True is not False


class TestJoined:
    def testjoined(self) -> None:
        assert True is not False


def test_collision_existing() -> None:
    assert True is not False


class TestCollision:
    def test_existing(self) -> None:
        assert True is not False


class TestUsesSelf:
    def test_self(self) -> None:
        assert self


class TestMultiWordName:
    def test_words(self) -> None:
        assert True is not False


class TestHTTPClient:
    def test_get(self) -> None:
        assert True


class TestMethodDocstring:
    def test_spread(self) -> None:
        """ Fails if the default axis changes
        """
        assert True


class TestDocumented:
    """Explains the group."""

    def test_one(self) -> None:
        assert True


class Builder:
    def test_shape(self) -> None:
        assert True


class TestBase:
    def test_shared(self) -> None:
        assert True


class TestDerived(TestBase):
    def test_more(self) -> None:
        assert True


class TestCommented:
    # Shared setup lives in conftest.
    def test_first(self) -> None:
        assert True  # trailing

    # Regression for issue 12: zero must fail.
    @pytest.mark.slow
    def test_zero(self) -> None:
        # inside
        assert True


class TestDetached:
    def test_first(self) -> None:
        assert True

    # A note apart from any method.

    def test_second(self) -> None:
        assert True


class TestHeader:  # grouped for the report
    def test_first(self) -> None:
        assert True


class TestQualname:
    def test_name(self) -> None:
        assert self.test_name.__qualname__ == "TestQualname.test_name"


class TestWithoutSelf:
    def test_bound_elsewhere(this) -> None:
        assert this
