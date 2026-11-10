def render(a: int):
    value = a + 1
    return t"{value}"


def plain_fstring(name: str) -> str:
    label = "ready"
    return f"{label}!"
