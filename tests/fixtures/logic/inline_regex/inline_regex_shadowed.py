from re import fullmatch

re = None


def shadowed(text):
    return fullmatch(r"v\d+", text)
