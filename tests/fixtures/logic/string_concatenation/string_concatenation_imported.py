from markupsafe import Markup
from pypdf import PageRange
from textwrap import dedent
from tags import make_tag


def label(number: int) -> str:
    return str(number)


def wrap(text: str, value) -> list[str]:
    return [
        Markup("<b>") + "<i>",
        "<i>" + Markup("<b>"),
        PageRange(slice(0, 5)) + "2:7",
        make_tag(text) + "!",
        "<b>" + label(3) + "!",
        "<b>" + dedent(text),
        "<b>" + value,
    ]
