import re


def stranded(lines):
    pattern = re.compile(r"#\s*(\d+)")
    return [line for line in lines if pattern.search(line)]


def version(text):
    version_re = re.compile(r"v\d+")
    if version_re.match(text):
        return version_re.pattern
    return None


def closure(text):
    scan = re.compile(r"q")

    def inner(item):
        return scan.match(item)

    return inner(text)


def rebound(text):
    scan = re.compile(r"s")
    scan = re.compile(r"t")
    return scan.match(text)


def deleted(text):
    scan = re.compile(r"s")
    del scan
    return text


def declared(text):
    scan = re.compile(r"u")

    def inner():
        nonlocal scan
        return scan

    return inner()


def read_by_call(text):
    scan = re.compile(r"p")
    return re.match(r"x", scan.pattern) and text


def debug_text(text):
    scan = re.compile(r"d")
    return f"{scan=}"


def commented(text):
    scan = re.compile(r"z")  # the z form
    return scan.match(text)


def explain(text):
    explanation = re.sub(r"a\d", "b", text)
    explanation = re.sub(r"c\d", "d", explanation)
    return explanation


def bound_in_a_block(text, flag):
    if flag:
        scan = re.compile(r"n")
        return scan.match(text)
    return None


def read_in_a_template(text):
    scan = re.compile(r"m")
    return t"{scan}", text


def other_nonlocal(text):
    scan = re.compile(r"o")
    count = 0

    def bump():
        nonlocal count
        count += 1

    bump()
    return scan.match(text), count
