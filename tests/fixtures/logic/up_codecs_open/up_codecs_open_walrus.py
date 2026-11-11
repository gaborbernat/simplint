import codecs


def read_walrus(path):
    with codecs.open(path, "r", name := "utf-8") as handle:
        return name, handle.read()


def read_keyword(path):
    with codecs.open(path, "r", encoding=(other := "utf-8")) as handle:
        return other, handle.read()


def read_buffered(path):
    with codecs.open(path, "r", "utf-8", "strict", size := 1) as handle:
        return size, handle.read()
