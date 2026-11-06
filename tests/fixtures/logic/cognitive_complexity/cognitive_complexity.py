def simple(value: int) -> int:
    if value:
        return 1
    return 0


def nested(items: list[int], limit: int) -> int:
    total = 0
    for item in items:
        if item > limit:
            if item % 2:
                total += 1
            elif item < 0:
                total -= 1
            else:
                total += 2
    return total


def boolean(a: bool, b: bool, c: bool) -> bool:
    return a and b or c


def handlers(items: list[str]) -> int:
    total = 0
    for item in items:
        try:
            total += int(item)
        except ValueError:
            if item:
                total -= 1
            else:
                total = 0
    return total


def comprehension(rows: list[list[int]], limit: int) -> list[int]:
    return [value for row in rows for value in row if value > limit if value % 2]


def ternary(items: list[int]) -> list[int]:
    return [item if item else -item for item in items]


def busy(a: int, b: int, c: int, items: list[int]) -> int:
    total = 0
    for item in items:
        if item > a:
            if item > b and item < c or item == 0:
                total += 1
            elif item < 0:
                total -= 1
            else:
                total += 2
        else:
            try:
                total += int(item)
            except ValueError:
                total = total if total else 0
    return total


def outer() -> int:
    def inner(value: int) -> int:
        if value:
            return 1
        return 0

    return inner(1)


def tangled(rows: list[list[int]], limit: int) -> int:
    total = 0
    for row in rows:
        for value in row:
            if value > limit:
                if value % 2:
                    total += 1
                elif value < 0:
                    total -= 1
                else:
                    while total > limit:
                        total -= 1
        if row and limit or total:
            total += 1
    return total
