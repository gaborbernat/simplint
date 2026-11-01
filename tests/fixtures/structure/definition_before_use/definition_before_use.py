import functools


def helper_used_by_main() -> int:
    return second_helper()


def second_helper() -> int:
    return 2


def main() -> int:
    return helper_used_by_main() + first_called()


def unrelated() -> int:
    return 3


def first_called() -> int:
    return 1


def runtime_helper() -> int:
    return 4


VALUE = runtime_helper()


class Service:
    def _private(self) -> int:
        return 1

    def run(self) -> int:
        return self._private() + self.other()

    def other(self) -> int:
        return 2

    @functools.cached_property
    def cached(self) -> int:
        return 3


def recursive(depth: int) -> int:
    return recursive(depth - 1) if depth else 0


def overloaded() -> int:
    return 1


def overloaded() -> int:  # noqa: F811
    return 2


def make() -> object:
    return Child()


class Base:
    pass


class Child(Base):
    pass


def deeper() -> int:
    return 5


def runtime_entry() -> int:
    return deeper()


RESULT = runtime_entry()


def ping(depth: int) -> int:
    return pong(depth)


def pong(depth: int) -> int:
    return ping(depth - 1) if depth else 0


class Sized:
    LIMIT = 3

    @property
    def size(self) -> int:
        return self._size

    @size.setter
    def size(self, value: int) -> None:
        self._size = value

    def _check(self, value: int = LIMIT) -> int:
        return value

    def apply(self) -> int:
        return self._check()


def outer_user() -> int:
    return commented_helper()


def unrelated_two() -> int:
    return 0


# explains the helper
def commented_helper() -> int:
    return 7


def lambda_helper() -> int:
    return 8


CALLBACK = lambda: lambda_helper()  # noqa: E731
