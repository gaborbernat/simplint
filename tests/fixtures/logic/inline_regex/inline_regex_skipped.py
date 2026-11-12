import re

_CHECK_PATTERN = re.compile(r"other")
MODULE_MATCH = re.match(r"\d+", "1")


class Table:
    ROW = re.compile(r"\w+")
    CELL = re.search(r"\d", "a1")


def plain_match(text):
    return re.match("abc", text)


def plain_split(text):
    return re.split("", text)


def plain_flagged(text):
    return re.match("abc", text, re.I)


def flags_from_name(text, flags):
    return re.search(r"v(\d+)", text, flags)


def commented(text):
    return re.search(
        r"v(\d+)",  # the version
        text,
    )


def check(text):
    return re.fullmatch(r"a+b", text)


def name_taken(text):
    return re.search(r"x\d", text)


lookup = lambda text: re.fullmatch(r"b+", text)
