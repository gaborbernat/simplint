from pathlib import Path


def _make_path(name):
    return Path(name)


def _build(key):
    return Path(key)


def load(name):
    callback = _make_path
    (callback)(name)


def table(key):
    registry = {}
    registry[key] = _build
    (registry)
    return registry


def _open(name):
    return open(name)


def read(name):
    return _open(name)
