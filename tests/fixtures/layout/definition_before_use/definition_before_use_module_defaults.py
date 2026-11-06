import random


def _pick(value: float = random.random()) -> float:
    return value


random.seed(1)


def run() -> float:
    return _pick()


def _plain(value: float = 0.5) -> float:
    return value


SCALE = 1


def walk() -> float:
    return _plain()
