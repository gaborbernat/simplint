import re

__all__ = ["LISTED"]

LISTED = re.compile(r"(a)(b)")


class Parser:
    LINE = re.compile(r"(\w+)=(\w+)")

    def value(self, text):
        return self.LINE.match(text).group(2)


def setup():
    global PAIR
    PAIR = re.compile(r"(\w+):(\w+)")


def pair_value(text):
    return PAIR.match(text).group(2)


SHARED = re.compile(r"(\w+)/(\w+)")


def reset():
    global SHARED
    SHARED = re.compile(r"(\w+)/(\w+)/")


def shared_value(text):
    return SHARED.match(text).group(2)


def outer(text):
    inner_pattern = re.compile(r"(\w+)!(\w+)")

    def swap():
        nonlocal inner_pattern
        inner_pattern = re.compile(r"(\w+)!")

    swap()
    return inner_pattern.match(text).group(2)


def control(text):
    return bool(re.match(r"(a)(b)", text))
