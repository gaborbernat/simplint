def greet(names: dict[str, str]) -> str:
    return "Hello " + names["first"] + " and " + names['last']


def quoted(names: dict[str, str]) -> str:
    return "Hello " + names["it's"]


def nested(prefix: str, width: int) -> str:
    return f"{prefix}{'a' + ('b' * width)}"
