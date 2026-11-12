import re
from re import match


def check(text):
    dollar = re.match(r"\d+$", text)
    absolute = re.match(r"\d+\Z", text)
    plain_string = re.match("\\d+\\Z", text)
    search = re.search(r"^\d+$", text)
    search_absolute = re.search(r"\A\d+\Z", text)
    grouped = re.match(r"(a|b)$", text)
    prefixed = re.match(r"^v\d+$", text)
    keywords = re.match(pattern=r"\d+$", string=text)
    imported = match(r"\d+$", text)
    return dollar, absolute, plain_string, search, search_absolute, grouped, prefixed, keywords, imported


def not_anchored(text):
    open_end = re.match(r"\d+", text)
    alternation = re.match(r"a|b$", text)
    flags = re.match(r"\d+$", text, re.MULTILINE)
    inline_flag = re.match(r"(?m)\d+$", text)
    escaped = re.match(r"\d+\$", text)
    in_class = re.match(r"[\d$]", text)
    search_open_start = re.search(r"\d+$", text)
    compiled = re.compile(r"\d+$").match(text)
    return open_end, alternation, flags, inline_flag, escaped, in_class, search_open_start, compiled
