def fits_once_inlined(items: list[str]) -> None:
    count = len(items)
    print(count)


def too_wide_once_inlined(items: list[str]) -> None:
    value = items[0] + items[1] + items[2] + items[3] + items[4] + items[5] + items[6]
    print(value, items[7], items[8], items[9])


def parenthesised_value(first: int, second: int) -> None:
    total = (first + second)
    print(total * 2)


def header_with_comment(items: list[str], verbose: bool) -> None:
    count = len(items)
    if verbose and count:  # the rows
        print(count)


def block_header(items: list[str]) -> None:
    count = len(items)
    for item in range(count):  # every row
        print(item)


def spread_value(items: list[str]) -> None:
    count = (
        len(items)
    )
    print(count)


def parenthesized_value_fits_bare(flag: bool) -> None:
    message = ("only the string without its parentheses fits on this one line.")
    check(flag, match=message)


def chained_in_call(session: Session) -> None:
    removed = (
        session.query(User)
        .filter(User.age > 2)
        .delete(synchronize_session=False)
    )
    check(removed, 3)


def chained_in_statement(session: Session) -> None:
    total = (
        session.first
        + session.second
    )
    print(total)
    other = (
        session.first
        + session.second
    )
    result = other * 2
    print(result)


def magic_trailing_comma(flag: bool) -> None:
    label = "short"
    check(flag, label,)


def numbers(flag: bool) -> None:
    label = "short"
    check(flag, label, 1 .real, 2.5)
