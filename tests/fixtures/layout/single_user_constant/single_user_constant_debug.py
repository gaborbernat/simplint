_WIDTH = 4
_SPAN = 5
print("loading")
_LIMIT = 30
_NAME = "a b"


def show() -> str:
    return f"{_LIMIT=}{_NAME=!s:>6}"


def wide() -> str:
    return f"""{_WIDTH
=}"""


def spans() -> str:
    return f"""{_SPAN
=}{_SPAN}"""
