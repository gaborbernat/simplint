class TestBox[T]:
    def test_holds_value(self) -> None:
        assert Box[int]().get(1) == 1

    def test_default(self, value: T | None = None) -> None:
        assert value is None


class TestBounded[T: int, *Ts, **P]:
    def test_sig(self, args: tuple[*Ts] | None = None, fn: Callable[P, int] | None = None) -> None:
        type Alias = list[T]
        assert Alias.__value__ == list[T]


class TestDefault[T]:
    def test_names_t(self, kind: type = list[T]) -> None:
        assert kind is list


class TestDefaulted[T: int = int]:
    def test_uses_default(self, value: T | None = None) -> None:
        assert value is None


class TestPlain:
    def test_plain(self) -> None:
        assert 1 == 1


class TestUnannotated[T]:
    def test_untyped(self, value=None) -> None:
        assert value is None
