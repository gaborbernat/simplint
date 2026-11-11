import codecs


def read_yield(path):
    with codecs.open(path, "r", "utf-8", (yield)) as handle:
        return handle.read()


def read_parenthesized_walrus(path):
    with codecs.open(path, "r", "utf-8", "strict", (size := 1)) as handle:
        return size, handle.read()
