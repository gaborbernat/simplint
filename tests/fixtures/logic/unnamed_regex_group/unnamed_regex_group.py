import re
import regex
from re import compile as compile_pattern

PARSER = re.compile(r"(\d+)\.(\d+)")


def pair(text):
    return re.match(r"(\d+)\.(\d+)", text)


def is_version(text):
    return bool(re.match(r"(\d+)\.(\d+)", text))


def missing(text):
    if re.search(r"(a)(b)", text) is None:
        return 1
    return 0


def present(text):
    if re.search(r"(a)(b)", text) is not None:
        return 1
    return 0


def full(text):
    if re.fullmatch(r"(x)(y)", text):
        return 2
    return 3


def first(text):
    found = re.match(r"(\d+)", text)
    return found.group(1) if found else None


def indexed_start(text):
    found = re.match(r"(a)", text)
    return found.start(1)


def loop_group(texts):
    for found in re.finditer(r"(a)", texts):
        print(found.group(1))


def split_one(text):
    return re.split(r"(\s+)", text)


def findall_index(text):
    words = re.findall(r"(\w)", text)
    return words[1]


def named(text):
    return re.match(r"(?P<major>\d+)\.(?P<minor>\d+)", text)


def named_angle(text):
    found = re.match(r"(?<a>x)(y)", text)
    return found.group(1)


def no_groups(text):
    return re.match(r"(?:\d+)\.(?=\d)(?i:\d+)(?<=a)(?<!b)(?!c)", text)


def classes(text):
    return re.match(r"[(]\(\d+\)[]()][^]x]", text)


def verbose_comment(text):
    return bool(re.match(r"(a) # (b)", text, re.VERBOSE))


def verbose_keyword(text):
    return bool(re.match(r"(a) # (b)", text, flags=re.X))


def verbose_combined(text):
    return bool(re.match(r"(a) # (b)", text, re.VERBOSE | re.IGNORECASE))


def verbose_value(text):
    return bool(re.match(r"(a) # (b)", text, 64))


def verbose_other_value(text):
    return bool(re.match(r"(a) # (b)", text, 4))


def verbose_regex(text):
    return bool(regex.match(r"(a) # (b)", text, regex.X))


def verbose_inline(text):
    return bool(re.match(r"(?x) a # (b)", text))


def not_verbose(text):
    return bool(re.match(r"(a) # (b)", text, re.IGNORECASE))


def bytes_fix(data):
    return bool(re.match(rb"(a)(b)", data))


def regex_module(text):
    return bool(regex.search(r"(a)(b)", text))


def concatenated(text):
    return bool(re.match(r"(a)" r"(b)", text))


def concatenated_escape(text):
    return bool(re.match(r"(a)" "(\\d)", text))


def fstring(text, prefix):
    return re.match(f"{prefix}(a)(b)", text)


def unterminated_class(text):
    return bool(re.match(r"(a)(b)[x", text))


def trailing_backslash(text):
    return bool(re.match("(a)(b)\\", text))


def unterminated_comment(text):
    return bool(re.match(r"(a)(b)(?#comment", text))


def imported_compile(text):
    return bool(compile_pattern(r"(a)(b)").match(text))


def purged():
    re.purge()


def module_pattern(text):
    return bool(PARSER.match(text))


def module_pattern_split(text):
    return PARSER.split(text)


def module_pattern_sub(text):
    return PARSER.sub("", text)


def pattern_groups(text):
    pattern = re.compile(r"(\w)(\w)")
    found = pattern.match(text)
    return found.groups()


def pattern_passed(text):
    return re.search(PARSER, text)


def pattern_rebound(text):
    pattern = re.compile(r"(a)(b)")
    return bool(pattern.match(text))


def other_pattern_rebound(text):
    pattern = re.compile(r"(c)")
    return pattern


def subscript_read(text):
    found = re.match(r"(a)(b)", text)
    return found[1]


def whole_match(text):
    found = re.match(r"(a)(b)", text)
    if found:
        return found.group(0) + found[0]
    return None


def span_only(text):
    found = re.match(r"(a)(b)", text)
    return found.span()


def named_dict(text):
    found = re.match(r"(a)(b)", text)
    return found.groupdict()


def last_index(text):
    found = re.match(r"(a)(b)", text)
    return found.lastindex


def printed_index(text):
    found = re.match(r"(a)(b)", text)
    print(found.lastindex)


def subscript_slice(items, text):
    found = re.match(r"(a)(b)", text)
    return items[found]


def substitute(text):
    return re.sub(r"(a)(b)", "x", text)


def substitute_backref(text):
    return re.sub(r"(a)(b)", r"\1\2", text)


def substitute_function(text):
    return re.sub(r"(a)(b)", lambda found: found.group(1), text)


def substitute_all(text):
    return re.subn(r"(a)(b)", "x", text)


def method_substitute(text, pattern):
    return PARSER.sub(r"\1", text)


def split_two(text):
    return re.split(r"(a)(b)", text)


def discarded(text):
    re.match(r"(a)(b)", text)


def unary_not(text):
    return not re.match(r"(a)(b)", text)


def boolean_operand(text, flag):
    return flag and re.match(r"(a)(b)", text)


def ternary_test(text):
    return 1 if re.match(r"(a)(b)", text) else 2


def ternary_value(text):
    return re.match(r"(a)(b)", text) if text else None


def elif_test(text, flag):
    if flag:
        return 1
    elif re.match(r"(a)(b)", text):
        return 2
    return 3


def while_test(texts):
    while re.match(r"(a)(b)", texts):
        texts = texts[1:]


def assert_test(text):
    assert re.match(r"(a)(b)", text)


def annotated(text):
    found: object = re.match(r"(a)(b)", text)
    return bool(found)


def chained_assign(text):
    first_match = second_match = re.match(r"(a)(b)", text)
    return first_match, second_match


def attribute_assign(holder, text):
    holder.found = re.match(r"(a)(b)", text)


def passed_to_print(text):
    print(re.match(r"(a)(b)", text))


def comprehension(items):
    return [item for item in items if re.match(r"(a)(b)", item)]


def returned(text):
    return re.match(r"(a)(b)", text)


def indirect_read(text):
    pattern = re.compile(r"(\w)")
    found = pattern.match(text)
    return found.group(1)


def conditional(text):
    return bool(re.match(r"(a)(?(1)b|c)(d)", text))


def named_backreference(text):
    return bool(re.match(r"(?P<n>a)(?P=n)(b)(c)", text))


def escaped_class(text):
    return bool(re.match(r"[\]\(](a)(b)", text))


NATURAL = re.compile(r"(\D*)(\d*)")


def natural_key(value):
    return [(text, int(digits or 0)) for text, digits in re.findall(r"(\D*)(\d*)", value)]


def natural_key_compiled(value):
    return [(text, int(digits or 0)) for text, digits in NATURAL.findall(value)]


PASSED_ON = re.compile(r"(\w+)=(\w+)")


def passed_on(value):
    return PASSED_ON.findall(value), print(PASSED_ON)
