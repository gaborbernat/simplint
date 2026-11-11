read = lambda: sorted(dir())


__all__ = ["check"]


def check() -> list[str]:
    return read()
