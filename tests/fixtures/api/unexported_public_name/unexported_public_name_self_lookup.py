import sys


def do_upper(value: str) -> str:
    return value.upper()


def do_lower(value: str) -> str:
    return value.lower()


def parse(name: str, value: str) -> str:
    return globals()["do_" + name](value)


__all__ = ["parse"]
