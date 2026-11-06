def parse_pairs(source):
    rows = source.splitlines()
    mapping = {}
    for row in rows:
        key, _, value = row.partition("=")
        mapping[key] = value
    total = len(mapping)
    if total == 0:
        raise ValueError("no values")
    return mapping, total
