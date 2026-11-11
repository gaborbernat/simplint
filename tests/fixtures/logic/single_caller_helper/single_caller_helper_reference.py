def register(table, key):
    table[key] = _build
    (table)
    return _seal(key)


def _build(key):
    return key.strip()


def _seal(key):
    value = key.upper()
    return value + "!"
