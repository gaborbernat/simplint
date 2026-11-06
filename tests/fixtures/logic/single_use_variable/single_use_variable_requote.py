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


def joined_in_field(items: list[str]) -> str:
    total = sum(
        1 for item in items if item
    )
    return f"{total} items"


def joined_and_requoted(items: list[str]) -> str:
    total = sum(
        1 for item in items if item.startswith("a")
    )
    return f"{total} items"


def kept_split_in_field(items: list[str]) -> str:
    total = sum(
        [1, 2,],
    )
    return f"{total} items"


def joined_operand(items: list[str]) -> str:
    total = (
        len(items)
        + 1
    )
    return f"{total * 2} items"
