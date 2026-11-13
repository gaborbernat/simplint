import weakref
from weakref import WeakMethod


def produce(rows: list[int]):
    try:
        yield rows[0]
        yield rows[1]
    finally:
        print("closed")


def first_from_next(rows: list[int]) -> None:
    stream = produce(rows)
    first = next(stream)
    print(first)


def first_from_default(rows: list[int]) -> None:
    stream = produce(rows)
    first = next(stream, None)
    print(first)


def reply_from_send(rows: list[int]) -> None:
    stream = produce(rows)
    reply = stream.send(None)
    print(reply)


def stop_from_close(rows: list[int]) -> None:
    stream = produce(rows)
    stream.close()
    print(rows)


async def first_from_anext(source: AsyncSource) -> None:
    stream = source.items()
    first = await anext(stream)
    print(first)


def collected(rows: list[int]) -> None:
    stream = produce(rows)
    items = list(stream)
    print(items)


class Scratch:
    def __init__(self) -> None:
        self.name = "scratch"

    def describe(self) -> str:
        return self.name


def make_scratch() -> Scratch:
    return Scratch()


def name_of_temporary() -> str:
    scratch = make_scratch()
    name = scratch.name
    return name.upper()


def method_on_temporary() -> str:
    scratch = make_scratch()
    return scratch.describe()


def part_of_literal() -> str:
    pair = ("scratch", 1)
    return pair[0]


def weak_target(scratch: Scratch) -> int:
    reference = weakref.ref(scratch)
    target = reference()
    print(target)
    return weakref.getweakrefcount(scratch)


def weak_method(scratch: Scratch) -> object:
    method = WeakMethod(scratch.describe)
    return method()
