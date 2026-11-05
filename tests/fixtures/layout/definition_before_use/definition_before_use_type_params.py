def _helper() -> int:
    return 1


def make[T: _Base]() -> int:
    return _helper()


class _Base:
    pass


class _Limit:
    pass


def _scale[T: _Limit = _Limit]() -> int:
    return 1


type _Items = list[_Node]


class _Node:
    pass
