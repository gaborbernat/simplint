import sys

CACHE = {}


def helper() -> int:
    return 1


def run() -> int:
    first = globals()["run"]
    second = vars().get("run")
    third = getattr(sys.modules[__name__], "run")
    fourth = getattr(sys.modules["other"], helper.__qualname__)
    fifth = getattr(run, helper.__qualname__)
    sixth = globals(1)[helper.__qualname__]
    seventh = len(helper.__qualname__)
    eighth = getattr(CACHE[__name__], helper.__qualname__)
    ninth = vars().items()
    return first, second, third, fourth, fifth, sixth, seventh, eighth, ninth


__all__ = ["run"]
