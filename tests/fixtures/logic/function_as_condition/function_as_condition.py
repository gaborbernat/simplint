import os

from unawaited_coroutine_lib import handler


def ready():
    return True


def matches(pattern, flag=False):
    return pattern, flag


def defaults(flag=False, *, strict=True):
    return flag, strict


@property
def decorated():
    return True


class Box:
    def is_full(self):
        return False

    @property
    def empty(self):
        return True

    @staticmethod
    def check():
        return True

    def run(self, other):
        if self.is_full:
            pass
        if self.empty:
            pass
        if self.check:
            pass
        if other.is_full:
            pass


def main(value):
    if ready:
        pass
    elif matches:
        pass
    elif value:
        pass
    while not ready and value:
        break
    assert defaults or value
    print(1 if handler else 2)
    if ready():
        pass
    if decorated:
        pass
    if os.path.exists:
        pass
    if value + 1:
        pass
