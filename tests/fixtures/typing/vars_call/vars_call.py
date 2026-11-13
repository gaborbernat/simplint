import argparse
from collections.abc import Callable


def options(parser: argparse.ArgumentParser) -> dict[str, str]:
    return vars(parser.parse_args())


def shadowed(vars: Callable[[], int]) -> int:
    return vars()


class Base:
    shared = 1

    @classmethod
    def own_names(cls) -> list[str]:
        return list(vars(cls))

    def __new__(cls):
        print(vars(cls))
        return super().__new__(cls)

    def type_names(self) -> list[str]:
        return list(vars(type(self))) + list(vars(self.__class__))


def class_names(klass: type[Base]) -> list[str]:
    return list(vars(klass)) + list(vars(Base))


def instance_names(item: Base) -> list[str]:
    return list(vars(item))


names_of = lambda klass: list(vars(klass))
