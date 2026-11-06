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


def walrus_and_star(value: list[int]) -> None:
    assert (ready := True, "walrus")
    assert ((ready := True), "parenthesized walrus")
    assert (*value, "starred test")
    assert (True, *value)


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


def generators(values: list[int]) -> None:
    assert (value for value in values)
    assert (value for value in values), "message"
    assert (compute() for value in values)


async def async_generators(values: list[int], stream: AsyncIterator[int]) -> None:
    assert (value async for value in stream)
    assert (await wait(value) for value in values)


def templates(value: str) -> None:
    assert "value {}".format(value)
    assert "{{".format()
    assert "a}}".format()
    assert "value %s" % value
    assert "100%% sure" % ()
    assert "%(name)s!" % {"name": value}
    assert "x %d%%" % (1,)
    assert "value {}".format(compute())
    assert "{}".format(value)
    assert "{value}".format(value=value)
    assert "%s" % value
    assert "%(name)s" % {"name": value}
    assert "{}" "x".format(value)
    assert f"{value}"


def computations(value: int) -> None:
    assert 1 + 1 == 2
    assert "a" * 3
    assert "a" + "b"
    assert not 0
    assert 2 * 3 - 1
    assert 1 < 2 < 3
    assert None == None
    assert 1 == True
    assert "a" < "b"
    assert -1
    assert 1 and "x"
    assert 0 or "x"
    assert "a" * 0 == ""
    assert not None
    assert 1 <= 2
    assert 2 > 1
    assert 2 >= 2
    assert "a" != "b"
    assert "a" != 1


def not_constant(value: int) -> None:
    assert 0 == 1
    assert "a" * 0
    assert 2 ** 3
    assert 1 / 2
    assert value + 1 == 2
    assert 9223372036854775807 + 1
    assert "a" * 5000
    assert "a" * 4096 + "b"
    assert 1 < "a"
    assert None < None
    assert None is None
    assert 1 in [1]
    assert "a" == 1
    assert 2 - 2
    assert not 1
    assert True and False
    assert value.format(1)
    assert 2 % value
    assert +1
    assert "%(key" % {}
    assert "x%" % ()
    assert "x{".format()
    assert "x}".format()
    assert "x%y" % ()
