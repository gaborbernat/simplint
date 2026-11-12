import re

import regex

REPLACEMENT = "_"
ESCAPED_REPLACEMENT = r"\d"


def split_plain(text):
    return re.split(",", text)


def split_maxsplit(text):
    return re.split(",", text, maxsplit=1)


def split_empty(text):
    return re.split("", text)


def sub_plain(text):
    return re.sub("-", "_", text)


def sub_name_replacement(text, repl):
    return re.sub("-", repl, text)


def sub_escape_without_fix(text):
    return re.sub("-", r"\n", text)


def sub_unescapable(text):
    return re.sub("-", r"\d", text)


def sub_count(text):
    return re.sub("-", "_", text, count=1)


def match_test(text):
    if re.match("abc", text):
        return True
    return False


def search_negated(text):
    return not re.search("abc", text)


def fullmatch_none(text):
    return re.fullmatch("abc", text) is None


def prefixmatch_bool(text):
    return bool(re.prefixmatch("abc", text))


def and_test(text):
    if text and re.search("abc", text):
        return True
    return False


def ternary(text):
    return 1 if re.match("abc", text) else 0


def comprehension(items):
    return [item for item in items if re.fullmatch("abc", item)]


def asserted(text):
    assert re.search("abc", text)


def guarded(value):
    match value:
        case str() if re.match("abc", value):
            return True
    return False


def match_value(text):
    found = re.match("abc", text)
    return found


def search_statement(text):
    re.search("abc", text)


def match_return(text):
    return re.match("abc", text)


def flags_plain(text):
    return re.match("abc", text, re.I)


def metacharacter_test(text):
    if re.match("a.c", text):
        return True
    return False


def regex_module(text):
    return regex.match("abc", text)


def sub_named_replacement(text):
    return re.sub("-", REPLACEMENT, text)


def sub_named_escape(text):
    return re.sub("-", ESCAPED_REPLACEMENT, text)


def set_comprehension(items):
    return {item for item in items if re.search("abc", item)}


def generator_condition(items):
    return any(item for item in items if re.search("abc", item))


def dict_comprehension(items):
    return {item: 1 for item in items if re.search("abc", item)}
