def total(items: list[int]) -> int:
    count = 0
    items.sort()
    print("sorted")
    for item in items:
        count += item
    return count


def read_later(items: list[int]) -> int:
    count = 0
    items.sort()
    return count + len(items)


def first(flag: bool) -> int:
    value = 1
    if flag:
        return 0
    return value


def chained(items: list[int]) -> list[int]:
    names = []
    items.sort()
    names.append("x")
    return names


def inner_only(items: list[int]) -> int:
    limit = 3
    size = len(items)
    return size + limit


def literals(items: list[int]) -> int:
    ready = not False
    offset = -1
    seen = {1, 2}
    table = {[1]: 2}
    items.sort()
    print(ready, offset, seen, table)
    return len(items)


def text_not_a_name(items: list[int]) -> list[int]:
    out = []
    items.sort()
    print("output")
    return out


def use_before_comment(items: list[int]) -> int:
    limit = 3
    items.sort()
    return len(items) + limit  # keeps the fix away


def use_before_lone_semicolon(items: list[int]) -> int:
    limit = 3
    items.sort()
    print(limit);
    return len(items)


def nested_binds_its_own(values: list[int]) -> object:
    total = 0
    print(values)
    print(total)

    def inner(items: list[int]) -> int:
        total = len(items)
        return total * 2

    return inner


def comprehension_binds_its_own(values: list[str]) -> list[str]:
    name = "x"
    print(values)
    print(name)
    return [name for name in values]
