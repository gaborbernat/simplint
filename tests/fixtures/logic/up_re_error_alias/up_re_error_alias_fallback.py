try:
    import regex as re
except ImportError:
    import re


def compile_pattern(text):
    try:
        return re.compile(text)
    except re.error:
        return None


def standard(text):
    import re as stdlib

    try:
        return stdlib.compile(text)
    except stdlib.error:
        return None
