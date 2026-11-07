from markupsafe import Markup, escape


def build() -> Markup:
    return Markup("<b>")


def f(text: str, safe: Markup) -> list[str]:
    held = Markup("<b>")
    return [
        held + "<i>",
        safe + "<i>",
        escape(text) + "<i>",
        Markup("<b>").strip() + "<i>",
        build() + "<i>",
        Markup("<b>").value + "<i>",
    ]
