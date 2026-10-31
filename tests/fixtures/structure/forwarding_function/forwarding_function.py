from collections.abc import Callable
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


def _configured(name: str) -> str:
    return str(name, encoding="utf-8")


def _keywords(name: str, *rest: str, **options: str) -> Path:
    return Path(name, *rest, encoding=options)


def more() -> tuple[str, Path]:
    return _configured("a"), _keywords("b")


def _render(name: str) -> str:
    return str(name, encoding="utf-8").strip()


def last() -> str:
    return _render("a")


def _apply(fn: Callable[[str], str], name: str) -> str:
    return fn(name)


def apply_all() -> str:
    return _apply(str.upper, "a")


def _make_lock(path: str, *, timeout: float = 0.1) -> str:
    return open_lock(path, timeout=timeout)


def use_lock() -> str:
    return _make_lock("a")
