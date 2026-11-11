def parse(text: str) -> int:
    return int(text)


class TestParse:
    def test_error_value(self) -> None:
        assert parse("1") == 1


class TestParseError:
    def test_value(self) -> None:
        assert parse("2") == 3


class TestParseWarning:
    def test_value(self) -> None:
        assert parse("3") == 4


class TestSame:
    def test_a(self) -> None:
        assert parse("4") == 4

    def testa(self) -> None:
        assert parse("5") == 5
