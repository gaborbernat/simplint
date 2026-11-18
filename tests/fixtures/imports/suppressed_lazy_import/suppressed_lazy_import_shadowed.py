ImportError = KeyError


def suppress(*_: object) -> object:
    return open("x")


with suppress(ImportError):
    lazy import msgpack
