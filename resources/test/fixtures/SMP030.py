from pathlib import Path


def _forward(name: str) -> Path:
    return Path(name)


def _callback(name: str) -> Path:
    return Path(name)


def _real(name: str) -> Path:
    path = Path(name)
    return path.resolve()


def use() -> list[Path]:
    return [_forward("a"), _real("b"), *map(_callback, ["c"])]


class Holder:
    def _make(self) -> Path:
        """Build it."""
        return Path()

    def build(self) -> Path:
        return self._make()


def _noop() -> None:
    pass


def caller() -> None:
    _noop()
