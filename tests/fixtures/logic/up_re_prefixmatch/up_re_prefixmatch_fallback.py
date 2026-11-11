try:
    import regex as re
except ImportError:
    import re


def first_version(text):
    return re.match(r"\d+", text)


def standard(text):
    import re as stdlib

    return stdlib.match(r"\d+", text)
