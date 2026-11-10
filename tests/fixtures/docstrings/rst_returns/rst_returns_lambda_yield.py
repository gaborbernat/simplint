from collections.abc import Iterator


def count(values: list[int]) -> int:
    """Count the values.

    :param values: The inputs.
    :returns: How many there are.
    """
    probe = lambda: (yield)  # noqa: E731
    return len(values) + bool(probe)


def numbers(limit: int) -> Iterator[int]:
    """Count up.

    :returns: The numbers.
    """
    probe = lambda: (yield)  # noqa: E731
    yield from range(limit) if probe else ()
