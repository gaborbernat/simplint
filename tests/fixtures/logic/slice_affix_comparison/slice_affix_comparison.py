def check(text, data, parts):
    prefix = text[:3] == "foo"
    suffix = text[-3:] == "bar"
    negated = text[:2] != "//"
    flipped = "ab" == text[:2]
    raw = data[:4] == b"\x89PNG"
    joined = (text + "x")[:1] == "y"
    attribute = parts.name[:5] == "socks"
    unicode = text[:2] == "é!"
    return prefix, suffix, negated, flipped, raw, joined, attribute, unicode


def not_affixes(text):
    wrong_length = text[:3] == "ab"
    middle = text[1:3] == "ab"
    stepped = text[:4:2] == "ab"
    variable = text[:3] == text
    return wrong_length, middle, stepped, variable
