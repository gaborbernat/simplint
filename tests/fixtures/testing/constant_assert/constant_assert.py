def check(value: int) -> None:
    assert True
    assert "message"
    assert value
    assert False, "unreachable"
    assert 0
    assert ()


def only() -> None:
    assert 1


def loops() -> None:
    for item in range(1):
        assert True
    else:
        pass
    while item:
        assert 1


def kinds() -> None:
    assert 1.5
    assert 0.0
    assert 1j
    assert b"bytes"
    assert ...


def tuples(value: int) -> None:
    assert (value, "value is set")
    assert (value, "value is set", "twice")


def split_messages(value: int) -> None:
    assert (value, ("value is "
                    "set"))
    assert (value,
            "value is "
            "set")


def containers(value: list[int]) -> None:
    assert [value]
    assert {1}
    assert {"key": value}
    assert list([value])
    assert [list()]
    assert f"value {value}"
    assert lambda: value
    assert [compute()]
    assert [*value]
    assert (*value,)
    assert []
    assert {}


def compute() -> int:
    return 1
