import io
import tarfile
from pathlib import Path


def read_text_file(path):
    with open(path, encoding="utf-8") as handle:
        return handle.read()


def spellings(path):
    with open(path, encoding="UTF8") as handle, open(path, encoding="utf_8") as other:
        return handle.read() + other.read()


def with_other_arguments(path):
    return open(path, errors="strict", encoding="utf8")


def io_functions(path):
    handle = io.open(path, encoding="utf-8")
    wrapped = io.TextIOWrapper(handle, encoding="utf-8")
    return wrapped.read()


def path_methods(path, text):
    Path(path).write_text(text, encoding="utf-8")
    return Path(path).read_text(encoding="utf-8")


def path_open(path):
    return path.open("w", encoding="utf-8")


def keyword_only(path):
    return Path(path).read_text(encoding="utf-8")


def other_encoding(path):
    return open(path, encoding="latin-1")


def bom_encoding(path):
    return open(path, encoding="utf-8-sig")


def dynamic_encoding(path, encoding):
    return open(path, encoding=encoding)


def positional_encoding(path):
    return open(path, "r", -1, "utf-8")


def module_function(path):
    return tarfile.open(path, encoding="utf-8")


def plain(path):
    return open(path)


def commented(path):
    return open(
        path,  # the file
        encoding="utf-8",
    )


def not_a_text_opener(path):
    return build(path, encoding="utf-8")


def build(path, encoding):
    return path, encoding
