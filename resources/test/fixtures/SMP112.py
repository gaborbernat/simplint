def greet(name: str, count: int) -> list[str]:
    return [
        "Hello, " + name + "!",
        "a" + "b",
        "{braces} " + name,
        f"{count} items for " + name,
        'single ' + name,
        "mixed " + name + ' quotes',
        "call " + str(count),
        "dict " + {"a": name}["a"],
        r"raw " + name,
    ]


DATA = b"a" + b"b"


def more(name: str) -> list[str]:
    return ["a" "b" + name, u"c" + name, """d""" + name]


def fstrings(name: str) -> list[str]:
    return [f"a" "b" + name, rf"c" + name]
