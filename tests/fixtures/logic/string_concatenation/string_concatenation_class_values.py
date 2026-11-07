class Markup(str):
    def __add__(self, other):
        return Markup(str.__add__(self, other))


def f(safe: Markup) -> list[str]:
    held = Markup("<b>")
    return [
        held + "<i>",
        safe + "<i>",
    ]
