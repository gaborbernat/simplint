def run() -> list[int]:
    return _cli_args(2)


def _cli_args(count: int) -> list[int]:
    kinds = _written_as(count)
    return [_config_of(index) for index in range(kinds)]


def _config_of(index: int) -> int:
    return index


def _written_as(value: int) -> int:
    return value


def outer() -> int:
    return _first() + _second()


def _second() -> int:
    return 2


def _first() -> int:
    return _first_nested()


def _first_nested() -> int:
    return 1
