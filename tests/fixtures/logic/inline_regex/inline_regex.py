import re
from re import fullmatch

import regex


def is_image(image_name: str) -> bool:
    return re.fullmatch(r"[a-z0-9]+(?:/[a-z0-9]+)*", image_name) is not None


class Parser:
    def value(self, text):
        return re.match(r"(\w+)=(\w+)", text).group(2)


def outer(text):
    def inner(line):
        return re.search(r"#\s*(\d+)", line)

    return inner(text)


def build(texts):
    return [re.findall(r"\d+", text) for text in texts]


def check(texts):
    return list(map(lambda text: re.finditer(r"a+b", text), texts))


def pick(items):
    return sorted(items, key=lambda item: re.split(r"[;,]", item)[0])


def stored(text):
    pattern = re.compile(r"(\w+)=(\w+)")
    return pattern.match(text)


def inline_compile(text):
    return re.compile(r"[a-z]+", re.IGNORECASE).fullmatch(text)


def byte_pattern(data):
    return re.match(rb"\d+", data, re.I)


def keyword_flags(text):
    return re.subn(r"\s+", " ", text, flags=re.MULTILINE)


def sub_count(text):
    return re.sub(r"\s+", " ", text, 1)


def sub_keyword_count(text):
    return re.sub(r"\s+", " ", text, count=2)


def verbose(text):
    return re.fullmatch(
        r"""
        (\d+)   # major
        \.(\d+) # minor
        """,
        text,
        re.VERBOSE | re.MULTILINE,
    )


def split_items(text):
    return re.split(r"[;,]", text, maxsplit=1)


def from_import(text):
    return fullmatch(r"v\d+", text)


def regex_module(text):
    return regex.search(r"x+", text)


def implicit(text):
    return re.match(r"(?:alpha)"
                    r"(?:beta)", text)


class Holder:
    COUNT = re.compile(r"\d+")

    def count(self, text):
        return self.COUNT.match(text)
