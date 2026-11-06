class Session:
    def run(self) -> int:
        return self._check() + self._step()

    def _step(self) -> int:
        return 1

    def _load(self) -> int:
        return 2

    def _check(self) -> int:
        return 3

    def __init__(self) -> None:
        self.value = self._load()

    @classmethod
    def open(cls) -> "Session":
        session = cls()
        return session._prepare()

    def _prepare(self) -> "Session":
        return self

    def __repr__(self) -> str:
        return str(self._step())


class Chain:
    def _inner(self) -> int:
        return 1

    def alpha(self) -> int:
        return self._outer()

    def beta(self) -> int:
        return self._inner()

    def _outer(self) -> int:
        return self._inner()
