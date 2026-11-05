def join(items: list[str]) -> str:
    sep = " ".join(items)
    return f"[{sep}]"


def quote(value: str) -> str:
    quoted = value.replace("'", "''")
    return f"'{quoted}'"


def nested(value: str) -> str:
    inner = f"<{value}>"
    return f"[{inner}]"


def triple(items: list[str]) -> str:
    sep = """ """.join(items)
    return f"[{sep}]"


def triple_fstring(items: list[str]) -> str:
    sep = f"""{items[0]}""".join(items)
    return f"[{sep}]"


def concatenated(value: str) -> str:
    inner = f"<{value}>" "tail"
    return f"[{inner}]"
