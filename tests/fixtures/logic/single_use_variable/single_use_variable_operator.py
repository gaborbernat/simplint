def copy_before(prices: list[int]) -> tuple[list[int], int]:
    last = prices.pop()
    return (prices + [0], last)


def percent(stack: list[int]) -> tuple[str, int]:
    top = stack.pop()
    return ("%s" % stack, top)


def emptied(queue: list[int]) -> tuple[bool, int]:
    head = queue.pop()
    return (not queue, head)


def member(config: dict[str, int]) -> tuple[bool, int]:
    value = config.pop("debug")
    return ("debug" in config, value)


def operator_holds_read(prices: list[int]) -> list[int]:
    last = prices.pop()
    return prices + [last]
