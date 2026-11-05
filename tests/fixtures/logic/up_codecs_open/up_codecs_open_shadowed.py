from codecs import open as codecs_open


def read_config(path):
    return codecs_open(path, "r", "utf-8").read()


def read_binary(path):
    return codecs_open(path, "rb", "utf-8").read()
