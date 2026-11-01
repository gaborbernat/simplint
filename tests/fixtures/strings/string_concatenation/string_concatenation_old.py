def greet(names: dict[str, str]) -> str:
    return "Hello " + str(names["first"]) + " and " + str(names['last'])


def quoted(names: dict[str, str]) -> str:
    return "Hello " + str(names["it's"])


def nested(prefix: str, width: int) -> str:
    return f"{prefix}{'a' + ('b' * width)}"
