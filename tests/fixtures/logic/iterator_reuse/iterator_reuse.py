from collections.abc import Iterator


def twice(numbers):
    texts = map(str, numbers)
    first = list(texts)
    second = sorted(texts)
    return first, second


def in_loop(rows, names):
    upper = (name.upper() for name in names)
    return [(row, list(upper)) for row in rows]


def nested_loop(rows, names):
    pairs = zip(names, rows)
    for _ in range(3):
        for pair in pairs:
            print(pair)


def parameter(lines: Iterator[str]) -> int:
    count = sum(1 for _ in lines)
    return count + len(list(lines))


def exclusive(numbers, flag):
    texts = map(str, numbers)
    if flag:
        return list(texts)
    return sorted(texts)


def branches(numbers, flag):
    texts = map(str, numbers)
    if flag:
        result = list(texts)
    else:
        result = sorted(texts)
    return result


def resumed(lines):
    rows = map(str.strip, lines)
    for row in rows:
        if not row:
            break
    return list(rows)


def resumed_from_inner_else(lines, stops):
    rows = map(str.strip, lines)
    for row in rows:
        for stop in stops:
            if row == stop:
                break
        else:
            break
    return list(rows)


def handed_over(numbers):
    texts = map(str, numbers)
    next(texts)
    return list(texts)


def rebuilt_each_time(rows, names):
    for _ in rows:
        upper = (name.upper() for name in names)
        print(list(upper))


def once(numbers):
    texts = map(str, numbers)
    return ",".join(texts)


def reassigned(lines: Iterator[str]):
    lines = list(lines)
    return len(lines), sorted(lines)


def keyed(names):
    upper = (name.upper() for name in names)
    return {name: list(upper) for name in names}


def both_in_one_arm(items, flag):
    remaining = map(str, items)
    if flag:
        first = list(remaining)
        second = list(remaining)
        return first, second
    return None


def nested_nonlocal(items):
    remaining = map(str, items)
    count = 0

    def bump():
        nonlocal count
        global LAST
        count += 1

    bump()
    return list(remaining), count
