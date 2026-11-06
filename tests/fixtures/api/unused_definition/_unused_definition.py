import functools

__all__ = ["_listed_unused"]


def _private_unused() -> int:
    return 1


def _listed_unused() -> int:
    return 3


def _shape_all(items: list[int]) -> list[int]:
    return [_Shaper()._udef_shape(item) for item in items]


@functools.cache
def _cached_unused() -> int:
    return 4


def _used_by_module() -> int:
    return 5


_USED = _used_by_module()


class _PrivateRecord:
    pass


class _Shaper:
    _UDEF_LIMIT = 3
    _UDEF_RATE = 2

    def _udef_shape(self, item: int) -> int:
        return min(item, self._UDEF_LIMIT) * self._UDEF_RATE

    def _udef_unused_method(self) -> int:
        return 6

    @property
    def _udef_view(self) -> int:
        return 7

    def __repr__(self) -> str:
        return "Shaper"
