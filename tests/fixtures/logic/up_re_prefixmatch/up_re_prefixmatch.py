import re
import re as regex
from re import match


def first_version(text):
    found = re.match(r"\d+\.\d+", text)
    return found.group(0) if found else None


def aliased(text):
    return regex.match(r"\d+", text)


def pattern_method(text):
    parser = re.compile(r"\d+")
    return parser.match(text)


def imported(text):
    return match(r"\d+", text)


def searched(text):
    return re.search(r"\d+", text)


def method_on_other(client, text):
    return client.match(text)


def patched(replacement):
    re.match = replacement
    del re.match
