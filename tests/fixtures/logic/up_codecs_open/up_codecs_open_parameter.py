import codecs


def read_shadowed(path, open=None):
    return codecs.open(path, "r", "utf-8").read()


def read_plain(path):
    return codecs.open(path, "r", "utf-8").read()
