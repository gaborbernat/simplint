def read_in_loop(items: list[int]) -> list[int]:
    out = []
    while items:
        print(out)
        out = [1]
        items.pop()
    print("done")
    return out


def captured(items: list[int]) -> list[int]:
    found = []

    def add() -> None:
        found.append(1)

    items.sort()
    add()
    return found


def names_read(items: list[int]) -> int:
    seed = 2
    items.sort()
    print(locals())
    return seed


def commented(items: list[int]) -> int:
    seed = 2  # the seed
    items.sort()
    return seed + len(items)


def rebound(items: list[int]) -> int:
    count = 0
    items.sort()
    for count in items:
        pass
    return count


def mutated_outside(items: list[int]) -> list[int]:
    try:
        out = []
        items.sort()
        out.append(1)
    except ValueError:
        return out
    return out


def value_reads_a_name(items: list[int]) -> int:
    base = len(items)
    items.sort()
    return base


def module_level_use(items: list[int]) -> int:
    total = 0
    if items:
        items.sort()
        total = 2
    return total


def string_names_it(items: list[int]) -> list[int]:
    out = []
    items.sort()
    print("out.append(1)")
    return out


def exec_by_string(items: list[int]) -> list[int]:
    lst = []
    six.exec_("lst.append(1)")
    assert lst == [1]
    return lst


def exec_method(items: list[int]) -> list[int]:
    out = []
    items.sort()
    runner.exec(items)
    return out


def reads_globals(items: list[int]) -> int:
    seed = 2
    items.sort()
    print(globals())
    return seed


def exec_after_use(items: list[int]) -> int:
    seed = 2
    items.sort()
    total = seed + len(items)
    items.exec_(total)
    return total
