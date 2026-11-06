# A copy with comments, docstrings and imports the duplicate check ignores.
import re


def parse_lines(text):
    """Parse the lines."""
    lines = text.splitlines()  # split once
    values = {}
    for line in lines:
        key, _, value = line.partition("=")
        values[key] = value
    count = len(values)
    if count == 0:
        raise ValueError("no values")
    return values, count
