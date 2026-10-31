import functools
import os.path as osp
from os import sep as separator

counter = 1


def decorate(function: object) -> object:
    return function


@decorate
class Holder(dict):
    counter = 2
    doubled = counter * 2

    def method(self, value: int = counter) -> int:
        return value + counter


def shadowed(counter: int) -> int:
    return counter


def nested() -> int:
    total = 0

    def inner() -> int:
        nonlocal total
        total += counter
        return total

    return inner()


def scopes(items: list[int]) -> list[int]:
    first = [counter + item for item in items]
    second = {counter for counter in items}
    third = {key: counter for key in items}
    fourth = (counter for _ in items)
    fifth = lambda value=counter: value + counter
    sixth = [[inner, *rest] for inner, *rest in [[1, 2, 3]]]
    if (named := counter) > 0:
        pass
    match items:
        case [head, *tail]:
            return [head, *tail, named]
        case {"key": found, **remaining}:
            return [found, len(remaining)]
    try:
        pass
    except ValueError as error:
        raise RuntimeError from error
    import json
    from json import dumps
    del json, dumps
    return [*first, *second, *third.values(), *fourth, fifth(), len(sixth), len(osp.sep), len(separator)]


functools.reduce(lambda left, right: left, [counter])
