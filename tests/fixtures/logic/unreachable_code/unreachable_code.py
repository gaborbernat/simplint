from contextlib import suppress
from typing import assert_never

import pytest


def after_return(text: str) -> int:
    return int(text)
    print("parsed")


def after_raise() -> None:
    raise ValueError
    print("never")


def after_branches(value: int) -> int:
    if value < 0:
        return -1
    elif value == 0:
        return 0
    else:
        return 1
    print(value)


def after_try(value: str) -> int:
    try:
        return int(value)
    except ValueError:
        raise
    print(value)


def after_try_else(value: str) -> int:
    try:
        number = int(value)
    except ValueError:
        return 0
    else:
        return number
    print(value)


def after_finally(value: str) -> int:
    try:
        print(value)
    finally:
        return 1
    print(value)


def after_forever(queue: list[str]) -> None:
    while True:
        queue.pop()
    print("done")


def after_jump(values: list[int]) -> None:
    for value in values:
        if value:
            continue
            print(value)
        break
        print(value)


def switched_off(value: int) -> int:
    if False:
        print(value)
    while 0:
        print(value)
    return value


def switched_off_else(value: int) -> int:
    if False:
        print(value)
    else:
        value += 1
    return value


def same_line(value: int) -> int:
    return value; print(value)


def with_block(items: list[int]) -> int | None:
    with suppress(IndexError):
        return items[0]
    return None


def forever_with_break(queue: list[str]) -> None:
    while True:
        if not queue:
            break
        queue.pop()
    print("done")


def branch_falls_through(value: int) -> int:
    if value:
        return 1
    print(value)
    return 0


def handler_falls_through(value: str) -> int:
    try:
        return int(value)
    except ValueError:
        print(value)
    return 0


def empty_generator():
    return
    yield


def guard_raise(queue: list[str]) -> None:
    while True:
        queue.pop()
    raise AssertionError("not reached")


def guard_assert(queue: list[str]) -> None:
    while True:
        queue.pop()
    assert False


def guard_fail(value: int) -> None:
    raise ValueError(value)
    pytest.fail("not reached")


def guard_never(value: int) -> None:
    return
    assert_never(value)


def guard_unreachable(value: int) -> None:
    return
    pytest.unreachable(value)


def nested_scopes() -> None:
    return

    def inner() -> None:
        global counter

    handler = lambda: (yield)


def declares() -> None:
    return
    global counter


def only_declarations() -> None:
    if False:
        from collections.abc import Sequence

        names: Sequence[str]


def switched_generator():
    if False:
        yield 1


if False:
    from collections.abc import Mapping
