def nested_walrus(f) -> tuple[int, int]:
    y = 0
    count = (y := f()) + 1
    return (y, count)


def plain_value(f) -> tuple[int, int]:
    y = 0
    count = f() + 1
    return (y, count)


def walrus_unread(f) -> tuple[int, int]:
    y = 0
    count = (z := f()) + 1
    return (y, count)
