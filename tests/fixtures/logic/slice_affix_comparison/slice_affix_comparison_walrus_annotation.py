def walrus_in_annotation(text):
    value: (alias := str) = text
    return value[:2] == "ab", alias
