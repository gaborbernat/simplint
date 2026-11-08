def render(markup: str, text: str, attrs: str) -> str:
    attrs_str = f" {attrs}"
    return f"{markup} {text}{attrs_str}"


def separated(name: str, other: str) -> str:
    sep = ", "
    return f"{name}{sep}{other}"


def braces(value: str) -> str:
    inner = "{value}"
    return f"<{inner}>"


def concatenated(value: str) -> str:
    inner = f"<{value}>" "tail"
    return f"[{inner}]"


def quote_kept(value: str) -> str:
    inner = f"it's {value}"
    return f'[{inner}]'


def raw_kept(value: str) -> str:
    inner = rf"\d{value}"
    return f"[{inner}]"


def outer_raw_kept(value: str) -> str:
    inner = f"a\t{value}"
    return rf"\n{inner}"


def conversion_kept(value: str) -> str:
    inner = f"<{value}>"
    return f"[{inner!r}]"


def concatenated_conversion(value: str) -> str:
    inner = f"<{value}>" "tail"
    return f"[{inner!r}]"
