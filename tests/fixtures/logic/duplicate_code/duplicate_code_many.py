def parse_lines(text):
    lines = text.splitlines()
    values = {}
    for line in lines:
        key, _, value = line.partition("=")
        values[key] = value
    count = len(values)
    if count == 0:
        raise ValueError("no values")
    return values, count
