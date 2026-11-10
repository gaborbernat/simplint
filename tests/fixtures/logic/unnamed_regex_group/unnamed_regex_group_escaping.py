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


def control(text):
    return bool(re.match(r"(a)(b)", text))
