from codecs import open


def read_config(path):
    return open(path, "r", "utf-8").read()
