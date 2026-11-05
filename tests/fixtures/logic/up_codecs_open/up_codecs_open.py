import codecs


def read_config(path):
    with codecs.open(path, "r", "utf-8") as handle:
        return handle.read()


def write_config(path, text):
    with codecs.open(filename=path, mode="w", encoding="utf-8", errors="replace", buffering=1) as handle:
        handle.write(text)


def read_default_mode(path):
    return codecs.open(path, encoding="utf-8").read()


def read_without_encoding(path):
    return codecs.open(path).read()


def read_none_encoding(path):
    return codecs.open(path, "r", None).read()


def read_binary(path):
    return codecs.open(path, "rb", "utf-8").read()


def read_dynamic_mode(path, mode):
    return codecs.open(path, mode, "utf-8").read()


def read_unpacked(*args):
    return codecs.open(*args)


def read_keywords(**kwargs):
    return codecs.open(**kwargs)


def read_commented(path):
    return codecs.open(path, "r", "utf-8").read()  # the old reader


def read_positional(path):
    return codecs.open(path, "r", "utf-8", "strict", 1).read()
