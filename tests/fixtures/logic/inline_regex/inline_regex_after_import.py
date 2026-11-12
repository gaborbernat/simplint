import re


def first(text):
    return re.search(r"a\d", text)


import os  # noqa: E402

print(os.sep)
