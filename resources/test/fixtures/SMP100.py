import re


def match(text: str) -> str:
    found = re.match("a", text)
    if found:
        return found.group(0)
    value = len(text)
    if value > 3 and text:
        return text
    other = text.strip()
    if not other:
        return ""
    reused = text.lower()
    if reused:
        pass
    return reused


def keep(text: str) -> str:
    pair = 1, 2
    if pair:
        return text
    commented = text  # keep this
    if commented:
        return commented
    late = text
    if text:
        return late
    return ""


class Body:
    value = 1
    if value:
        pass


def shapes(text: str) -> object:
    bare = text.strip()
    if bare:
        pass
    total = len(text) + 1
    if total > 3:
        pass
    again = text.lower()
    if again > "a":
        return again
    first, second = text, text
    if first:
        return second
    called = text.upper()
    if len(called):
        return called
    return None
