import os
from typing import Any

LIMIT = len(os.sep)


def inlined_into_call(items: list[str]) -> None:
    count = len(items)
    print(count)


def inlined_into_return(prices: list[float]) -> str:
    total = sum(prices)
    return f"{total:.2f}"


def inlined_with_parentheses(first: int, second: int, flag: bool) -> None:
    total = first + second
    print(total * 2)


def inlined_tuple(first: int, second: int) -> None:
    pair = first, second
    print(pair)


def inlined_into_loop_header(items: list[str]) -> None:
    count = len(items)
    for item in range(count):
        print(item)


def inlined_after_store(items: list[str], flags: dict[int, int]) -> None:
    total = len(items)
    flags[0] = total


def inlined_before_effect(items: list[str], log: Any) -> None:
    count = len(items)
    log.write(count)


def inlined_after_call(items: list[str], log: Any) -> None:
    count = len(items)
    print(log.name, count)


def inlined_after_plain_names(items: list[str], other: int) -> None:
    count = len(items)
    print(other, count)


def inlined_before_conditional(items: list[str], flag: bool) -> None:
    count = len(items)
    print(count and flag)


def inlined_with_ternary_test(items: list[str], flag: bool) -> None:
    count = len(items)
    print(1 if count else 0)


def inlined_as_argument(first: int, second: int) -> None:
    total = first + second
    print(total)


def kept_global(items: list[str]) -> None:
    global TOTAL
    TOTAL = len(items)
    print(TOTAL)


def kept_underscore(items: list[str]) -> None:
    _ = len(items)
    print(_)


def kept_dunder(items: list[str]) -> None:
    __value__ = len(items)
    print(__value__)


async def kept_await(source: Any) -> None:
    data = await source.read()
    print(data)


def kept_walrus(items: list[str]) -> None:
    count = (size := len(items))
    print(size, count)


def kept_yield(items: list[str]):
    count = len(items)
    yield count


def kept_return_name(items: list[str]) -> int:
    count = len(items)
    return count


def kept_read_twice(items: list[str]) -> None:
    count = len(items)
    print(count)
    print(count)


def kept_read_later(items: list[str]) -> None:
    count = len(items)
    other = 1
    print(count, other)


def kept_read_in_nested_block(items: list[str], verbose: bool) -> None:
    count = len(items)
    if verbose:
        print(count)


def kept_read_in_elif(items: list[str], verbose: bool) -> None:
    count = len(items)
    if verbose:
        print("verbose")
    elif count > 1:
        print("many")


def shared_line(items: list[str]) -> None:
    count = len(items); print(count)


def kept_read_in_loop_body(items: list[str]) -> None:
    count = len(items)
    for item in items:
        print(item, count)


def kept_read_in_comprehension(items: list[str]) -> list[int]:
    count = len(items)
    return [count for _ in items]


def kept_read_in_lambda(items: list[str]) -> Any:
    count = len(items)
    return lambda: count


def kept_read_in_while_test(items: list[str]) -> None:
    count = len(items)
    while count:
        count -= 1


def kept_read_in_if_test(items: list[str]) -> None:
    found = items.index("x")
    if found:
        print(found)


def kept_read_by_nested_def(items: list[str]) -> None:
    count = len(items)

    def show() -> None:
        print(count)

    show()


def kept_locals(items: list[str]) -> None:
    count = len(items)
    print(locals(), count)


def kept_rebound_by_import(items: list[str]) -> None:
    count = len(items)
    from os import count
    print(count)


def kept_rebound_by_def(items: list[str]) -> None:
    count = len(items)

    def count() -> int:
        return 1

    print(count)


def kept_rebound_by_class(items: list[str]) -> None:
    count = len(items)

    class count:
        pass

    print(count)


def kept_rebound_by_except(items: list[str]) -> None:
    count = len(items)
    try:
        print(items)
    except ValueError as count:
        print(count)


def kept_rebound_by_match(items: list[str]) -> None:
    count = len(items)
    match items:
        case [count]:
            print(count)


def kept_rebound_by_star(items: list[str]) -> None:
    count = len(items)
    match items:
        case [*count]:
            print(count)


def kept_rebound_by_mapping(items: dict[str, int]) -> None:
    count = len(items)
    match items:
        case {"a": 1, **count}:
            print(count)


def kept_rebound_by_import_as(items: list[str]) -> None:
    count = len(items)
    import json as count
    print(count)


def kept_rebound_by_import_dotted(items: list[str]) -> None:
    count = len(items)
    import count.sub
    print(count)


def kept_rebound_by_import_plain(items: list[str]) -> None:
    count = len(items)
    import count
    print(count)


def kept_comment(items: list[str]) -> None:
    count = len(items)  # the rows
    print(count)


def kept_leading_if(text: str) -> None:
    found = text.find("x")
    if found:
        print(found)


def kept_module_level_name() -> None:
    print(LIMIT)


def kept_tuple_target(items: list[str]) -> None:
    first, second = items[0], items[1]
    print(first, second)


def joined_multiline_value(items: list[str]) -> None:
    count = (
        len(items)
    )
    print(count)


def kept_mention_in_value(items: list[str]) -> None:
    count = 0
    count = len(items) + count
    print(count)


def kept_exception_message(items: list[str]) -> None:
    msg = f"{len(items)} items"
    raise ValueError(msg)


def kept_type_parameter(items: list[str]) -> None:
    bound = list[str]

    def first[T: bound](value: T) -> T:
        return value

    print(first(items))


class Box:
    count = len("items")
    print(count)

    def method(self, items: list[str]) -> None:
        total = len(items)
        print(total)


def kept_split_value(items: list[str]) -> None:
    count = [
        len(items),
    ]
    print(count)


def kept_yield_tuple(first: int, second: int):
    pair = first, second
    yield pair


def inlined_yield_from_tuple(first: int, second: int):
    pair = first, second
    yield from pair


def inlined_yield_from(items: list[str]):
    names = sorted(items)
    yield from names


calls = 0


def bump() -> int:
    global calls
    calls += 1
    return calls


def unsafe_before_formatted_name(stack: list[int]) -> str:
    top = stack.pop()
    return f"{stack} after popping {top}"


def unsafe_before_double_star(base: dict[str, int], extra: dict[str, int]) -> dict[str, int]:
    added = base.update(extra) or len(base)
    return {**base, "n": added}


def unsafe_before_star(items: list[int]) -> None:
    head = items.pop()
    print([*items], head)


def unsafe_before_keyword_unpacking(options: dict[str, int]) -> None:
    removed = options.pop("a")
    print(**options, removed=removed)


def unsafe_before_global_name() -> None:
    value = bump()
    print(calls, value)


def unsafe_before_nonlocal_name() -> tuple[int, int]:
    total = 0

    def step() -> int:
        nonlocal total
        total += 1
        return total

    value = step()
    return total, value


def inlined_after_parameter(items: list[int], log: list[int]) -> None:
    count = len(items)
    print(log, count)


def inlined_pure_after_formatted_name(stack: list[int], top: int) -> str:
    shown = top + 1
    return f"{stack} after popping {shown}"
