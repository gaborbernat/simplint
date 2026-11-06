def mapping(a: str, b: int) -> dict[str, int]:
    values = {}
    values["x"] = 1
    values["y"] = 2
    return values


def items(a: str, b: int) -> list[str]:
    names = []
    names.append(a)
    names.append(b)
    return names


def unique() -> set[int]:
    seen = set()
    seen.add(1)
    seen.add(2)
    return seen


def keyed(name: str, value: int) -> dict[str, int]:
    result = {}
    result[name] = value
    return result


def tuple_key() -> dict[tuple[int, int], tuple[int, int]]:
    grid = {}
    grid[1, 2] = 3, 4
    return grid


def duplicate() -> dict[str, int]:
    table = {}
    table["k"] = 1
    table["k"] = 2
    return table


def hashes_alike() -> dict[int, str]:
    table = {}
    table[1] = "a"
    table[True] = "b"
    return table


def repeated_name(key: str) -> dict[str, int]:
    table = {}
    table[key] = 1
    table[key] = 2
    return table


def repeated_element() -> set[float]:
    seen = set()
    seen.add(0)
    seen.add(0.0)
    return seen


def calls() -> dict[str, int]:
    table = {}
    table["k"] = compute()
    table["j"] = 1
    return table


def reads_itself(name: str) -> dict[str, object]:
    table = {}
    table["self"] = table
    table[name] = 1
    return table


def single() -> list[int]:
    out = []
    out.append(1)
    return out


def other_target(a: str) -> tuple[dict[str, int], int]:
    table = {}
    count = 1
    return table, count


def other_key(a: str) -> dict[str, int]:
    table = {}
    a["x"] = 1
    return table


def not_a_call(a: str) -> list[str]:
    names = []
    a
    return names


def not_a_method(a: str) -> list[str]:
    names = []
    print(a)
    return names


def wrong_method(a: str, b: str) -> list[str]:
    names = []
    names.extend([a, b])
    return names


def wrong_receiver(a: str, b: str) -> list[str]:
    names = []
    b.append(a)
    return names


def too_many(a: str, b: str) -> list[str]:
    names = []
    names.append(a, b)
    return names


def compute() -> int:
    return 1
