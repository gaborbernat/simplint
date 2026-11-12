import re


def outer(text):
    return re.match(r"a\d", re.search(r"b\d", text))
