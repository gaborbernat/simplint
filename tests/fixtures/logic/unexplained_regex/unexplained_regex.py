import re

import regex


def named(text):
    return bool(re.match(r"(?P<a>x)(?P<b>x)(?P<c>x)(?P<d>x)", text))


def non_capturing(text):
    return bool(re.match(r"(?:x)(?:x)(?:x)(?:x)", text))


def lookaround(text):
    return bool(re.match(r"(?=x)(?!x)(?<=x)(?<!x)", text))


def atomic(text):
    return bool(re.match(r"(?>x)(?>x)(?>x)(?>x)", text))


def conditional(text):
    return bool(re.match(r"(?(1)a|b)(?(1)a|b)", text))


def alternation(text):
    return bool(re.match(r"a|b|c|d|e", text))


def classes(text):
    return bool(re.match(r"[a][b][c][d]", text))


def group_repeat(text):
    return bool(re.match(r"(a)(b)(c)*", text))


def group_lazy_repeat(text):
    return bool(re.match(r"(a)(b)(c)+?", text))


def group_bounded_repeat(text):
    return bool(re.match(r"(a){2,3}(b)(c)", text))


def class_repeat(text):
    return bool(re.match(r"[a]{2,3}[b]+[c]*[d]?", text))


def class_repeat_short(text):
    return bool(re.match(r"(a)(b)[c]{2,3}", text))


def separator(text):
    return re.split(r"[ \t]*,[ \t]*", text)


def class_alone(text):
    return bool(re.match(r"(a)(b)[c]", text))


def escapes(text):
    return bool(re.match(r"(a)(b)(c)\(\d+\)", text))


def class_with_metacharacters(text):
    return bool(re.match(r"[|(](a)[|(]", text))


def inline_verbose(text):
    return bool(re.match(r"(?x)(a)(b)(c)(d)", text))


def inline_verbose_commented(text):
    return bool(re.match(r"(?x) (a) (b) (c) # the last", text))


def flags_combined(text):
    return bool(re.match(r"(a)(b)(c)(d)", text, re.X | re.I))


def flags_keyword(text):
    return bool(re.match(r"(a)(b)(c)(d)", text, flags=re.VERBOSE))


def regex_module(text):
    return bool(regex.search(r"(a)(b)(c)(d)", text))


def bytes_pattern(data):
    return bool(re.match(rb"(a)(b)(c)(d)", data))


def concatenated(text):
    return bool(re.match(r"(a)" r"(b)" "(c)" r"(d)", text))


def fstring(text, prefix):
    return bool(re.match(f"{prefix}(a)(b)(c)(d)", text))


def unparsable(text):
    return bool(re.match(r"(a)(b)(c)(d)[x", text))


def comment_group(text):
    return bool(re.match(r"(a)(b)(c)(d)(?#the four)", text))


def flags_only(text):
    return bool(re.match(r"(?i)(a)(b)(c)", text))


def hash_in_class(text):
    return bool(re.match(r"(a)(b)(c)[#]", text, re.VERBOSE))


def verbose_commented(text):
    return bool(re.match(r"(a)(b)(c)(d)  # the four", text, re.VERBOSE))


def verbose_short(text):
    return bool(re.match(r"(a)(b)(c)", text, re.VERBOSE))


def digits(text):
    return bool(re.match(r"(a)(b)(c)\d+\w+", text))


def backreference(text):
    return bool(re.match(r"(?P<a>x)(?P=a)(?P<b>y)(?P<c>z)", text))


PATTERN = re.compile(r"(a)(b)(c)(d)")


def literal_braces(text):
    return bool(re.match(r"(a)(b)(c){1,2,3}", text))
