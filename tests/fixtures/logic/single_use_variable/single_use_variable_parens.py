def delims(markup: str) -> tuple[str, str]:
    is_percent = markup == "{{%"
    return ("{{%", "%}}") if is_percent else ("{{<", ">}}")


def either_in_and(flag: bool, first: bool, second: bool) -> bool:
    either = first or second
    return either and flag


def negated_in_and(flag: bool, other: bool) -> bool:
    negated = not other
    return negated and flag


def comparison_in_and(items: list[str], flag: bool) -> bool:
    empty = len(items) == 0
    return empty and flag


def sum_in_product(first: int, second: int) -> int:
    total = first + second
    return total * 2


def conditional_in_sum(flag: bool, first: int, second: int) -> int:
    chosen = first if flag else second
    return chosen + 1


def either_in_or(flag: bool, first: bool, second: bool) -> bool:
    either = first or second
    return either or flag


def bitwise_or(first: int, second: int) -> int:
    part = first ^ second
    return part | second


def bitwise_xor(first: int, second: int) -> int:
    part = first | second
    return part ^ second


def bitwise_and(first: int, second: int) -> int:
    part = first | second
    return part & second


def shifted(first: int, second: int) -> int:
    part = first + second
    return part << second


def powered(first: int, second: int) -> int:
    part = first + second
    return part ** second


def inverted(first: int, second: int) -> bool:
    same = first == second
    return not same


def negated_sum(first: int, second: int) -> int:
    total = first + second
    return -total
