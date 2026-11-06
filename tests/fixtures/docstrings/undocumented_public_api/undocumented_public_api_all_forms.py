__all__ = ["one"] + ["two"]
__all__ += (extra := ["three"])
__all__ += list({"four"})
__all__ += list(["five"], [])
__all__ += list(extra)


def one() -> None: ...


def two() -> None: ...
