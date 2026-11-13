class Markup(str):
    def __add__(self, other: str) -> "Markup":
        return Markup(str.__add__(self, other.replace("<", "&lt;")))


class Label:
    def __init__(self, text: str) -> None:
        self.text = text


SAFE = Markup("<b>") + "<i>"
PREFIXED = "<i>" + Markup("<b>")
named = Markup("<b>")
ESCAPED = named + "<i>"


def wrap(text: str) -> list[str]:
    return [Label("x") + "!", "<b>" + text, str(text) + "!", "<b>" + "<i>" + text]


def shadowed() -> str:
    class Label:
        text = "x"

    return "name: " + Label().text
