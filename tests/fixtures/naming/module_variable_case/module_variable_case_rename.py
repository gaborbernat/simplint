import functools

counter = 1


def decorate(function: object) -> object:
    return function


@decorate
class Holder(dict):
    doubled = counter * 2

    def method(self, value: int = counter) -> int:
        return value + counter


def nested() -> int:
    total = 0

    def inner() -> int:
        nonlocal total
        total += counter
        return total

    return inner()


def bump() -> None:
    global counter
    counter += 1
    del counter


def scopes(items: list[int]) -> list[int]:
    first = [counter + item for item in items]
    third = {key: counter for key in items}
    fifth = lambda value=counter: value + counter
    if (named := counter) > 0:
        pass
    match items:
        case [head, *tail] if head == counter:
            return [head, *tail, named]
    return [*first, *third.values(), fifth()]


functools.reduce(lambda left, right: left + counter, [counter])


def generators(items: list[int]) -> list[int]:
    return [first + counter for first in items for second in items if second] + [0 for Holder.doubled in items]
