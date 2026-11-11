import codecs

READERS = [codecs.open(open, "r", "utf-8") for open in ()]


def read(path):
    def helper(open):
        return open

    return codecs.open(path, "r", "utf-8").read()
