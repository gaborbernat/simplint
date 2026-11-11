_MISSING = object()
_SHARED = object()
_SHARED = 2
_TARGET = _TARGET_TWO = object()
_ARGUMENT = object(1)
_CALLED = dict()
_ANNOTATED: object = object()
MAYBE = None
if MAYBE is None:
    _IN_IF = object()


class Config:
    SENTINEL = object()
    value = object()

    def method(self):
        local = object()
        return local


def run():
    _LOCAL = object()
    return _LOCAL


_LOCAL_STRING = "object()"


class Outer:
    class Inner:
        LIMIT = object()


def make_local():
    class Local:
        MARK = object()

    return Local


class Reassigned:
    MISSING = object()
    MISSING = None


class Shadowed:
    LIMIT = object()


LIMIT = 1
LIMIT = 2


class Method:
    MISSING = object()

    def get(self, key):
        MISSING = self.lookup(key)
        return MISSING


class Nested:
    LIMIT = object()

    class Inner:
        LIMIT = 3


class Comprehension:
    DEFAULT = object()
    names = [DEFAULT for DEFAULT in range(3)]
