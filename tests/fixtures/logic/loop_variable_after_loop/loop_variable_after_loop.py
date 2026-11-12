from collections.abc import Callable


def last_row(rows: list[str]) -> str:
    for row in rows:
        print(row)
    return row


def unpacked(pairs: list[tuple[str, int]]) -> int:
    for name, count in pairs:
        print(name)
    print(name)
    return count


def nested_read(rows: list[str]) -> None:
    for row in rows:
        print(row)
    if rows:
        print(row.upper())


def searched(values: list[int]) -> int | None:
    for value in values:
        if value < 0:
            break
    else:
        return None
    return value


def initialised(values: list[int]) -> int | None:
    value = None
    for value in values:
        print(value)
    return value


def else_binds(values: list[int]) -> int:
    for value in values:
        print(value)
    else:
        value = 0
    return value


def rebound(values: list[int]) -> int:
    for value in values:
        print(value)
    value = 0
    return value


def comprehension(values: list[int]) -> list[int]:
    for value in values:
        print(value)
    return [value * 2 for value in values]


def captured(values: list[int]) -> Callable[[], int]:
    for value in values:
        print(value)

    def read() -> int:
        return value

    return read


def by_name(values: list[int]) -> dict[str, int]:
    for value in values:
        print(value)
    return locals()


def inner_break(groups: list[list[int]]) -> int:
    for group in groups:
        for item in group:
            break
    return group


for entry in []:
    print(entry)
print(entry)
