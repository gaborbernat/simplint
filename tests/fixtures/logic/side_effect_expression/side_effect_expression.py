def comprehension(users: list[str]) -> None:
    [print(user) for user in users]


def filtered(groups: list[list[str]]) -> None:
    {print(user) for group in groups if group for user in group if user}


def keyed(users: list[str]) -> None:
    {user: print(user) for user in users}


async def awaited(tasks: list) -> None:
    [await task for task in tasks]


def conjunction(verbose: bool) -> None:
    verbose and print("sent")


def disjunction(made: bool) -> None:
    made or print("missing")


def negated_group(made: bool, ready: bool) -> None:
    (made and ready) or print("missing")


def three(made: bool, ready: bool) -> None:
    made or ready or print("missing")


def chained(made: bool, ready: bool) -> None:
    made and ready and print("ready")


def compared(made: int) -> None:
    made == 1 or print("missing")


def comprehension_same_line(users: list[str]) -> None:
    [print(user) for user in users]; print("x")


def same_line(made: bool) -> None:
    made and print("x"); print("y")


def commented(made: bool) -> None:
    made and print(  # why
        "x"
    )


def generator(made: bool):
    made and (yield 1)


def pure(values: list[int], flag: bool) -> None:
    print([value for value in values])
    result = flag and print("x")
    print(result)
    print("x") and flag
