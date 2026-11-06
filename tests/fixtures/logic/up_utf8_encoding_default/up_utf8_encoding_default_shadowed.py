def open(path, encoding):
    return path, encoding


def read_text_file(path):
    return open(path, encoding="utf-8")
