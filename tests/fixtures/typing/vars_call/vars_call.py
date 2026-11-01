import argparse
from collections.abc import Callable


def options(parser: argparse.ArgumentParser) -> dict[str, str]:
    return vars(parser.parse_args())


def shadowed(vars: Callable[[], int]) -> int:
    return vars()
