def build(val: str, cur_ends_with_sep: bool, ends_with_sep: bool, sep: str) -> str:
    ending = f" {sep}" if cur_ends_with_sep else ""
    prepend = "  " if ends_with_sep else ""
    return f"{prepend}{val}{ending}"


def conditional_in_field(first: int, second: int, flag: bool) -> str:
    total = first if flag else second
    return f"{total}"


def name_in_field(value: str) -> str:
    label = value
    return f"{label}"


def conditional_outside_field(first: int, second: int, flag: bool) -> None:
    total = first if flag else second
    print(total)
