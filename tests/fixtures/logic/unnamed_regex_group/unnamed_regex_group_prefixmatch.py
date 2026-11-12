import re


def has_pair(text):
    return re.prefixmatch(r"(\w+)=(\w+)", text) is not None
