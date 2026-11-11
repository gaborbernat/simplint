from pathlib import Path


def load_a(name):
    return _make_a\
(name)


def _make_a(name):
    return Path(name)


def load_b(name):
    return (_make_b
            (name))


def _make_b(name):
    return Path(name)


def load_c(name):
    return _make_c  (name)


def _make_c(name):
    return Path(name)


def load_d(name):
    return [_make_d  # why
            (name)]


def _make_d(name):
    return Path(name)
