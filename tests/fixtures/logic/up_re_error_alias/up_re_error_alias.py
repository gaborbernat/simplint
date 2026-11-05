import re
from re import error


def compile_pattern(text):
    try:
        return re.compile(text)
    except re.error:
        return None


def fail(message):
    raise re.error(message)


def imported(text):
    try:
        return re.compile(text)
    except error:
        return None
