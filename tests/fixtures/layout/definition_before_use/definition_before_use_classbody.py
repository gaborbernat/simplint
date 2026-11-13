class Interrupted(Exception):
    pass


class Session:
    Interrupted = Interrupted


def main() -> type:
    return Interrupted


def _describe(value: object) -> str:
    return f"<{value}>"


class Box:
    def __repr__(self) -> str:
        return _describe(self)

    __str__ = __repr__


def _measure(value: object) -> int:
    return len(str(value))


class Ruler:
    def _get(self) -> int:
        return _measure(self)

    size = property(_get)


def _token(value: int) -> int:
    return value


class Tokens:
    def make(value: int) -> int:
        return _token(value)

    first = make(1)
