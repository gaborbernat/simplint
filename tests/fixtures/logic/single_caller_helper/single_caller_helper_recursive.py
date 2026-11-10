def _countdown(count):
    if count:
        return _countdown(count - 1)
    return 0
