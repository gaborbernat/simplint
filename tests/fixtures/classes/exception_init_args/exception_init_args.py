import argparse

__all__ = []


class Fixed(Exception):
    def __init__(self):
        super().__init__("fixed message")


class Dropped(Exception):
    def __init__(self, path):
        super().__init__()
        self.path = path


class Keyword(Exception):
    def __init__(self, message, *, code):
        super().__init__(message)
        self.code = code


class Explicit(ValueError):
    def __init__(self, path, line):
        ValueError.__init__(self, f"{path}:{line}")


class Qualified(argparse.ArgumentTypeError):
    def __init__(self, value, kind):
        argparse.ArgumentTypeError.__init__(self, f"{value}: {kind}")


class Generic(ValueError):
    def __init__(self, path, line):
        Exception.__init__(self, path)


class TwoArgumentSuper(Exception):
    def __init__(self, path, line):
        super(TwoArgumentSuper, self).__init__(path)


class FoundAtRuntime(Exception):
    def __init__(self, path, line):
        type(self).__bases__[0].__init__(self, path)


class Optional(Exception):
    def __init__(self, path, line=0):
        super().__init__(path)


class Forwarded(Exception):
    def __init__(self, path, line):
        super().__init__(path, line)


class Variadic(Exception):
    def __init__(self, *args):
        super().__init__(*args)


class Keyed(Exception):
    def __init__(self, message, *, code=0):
        super().__init__(message)


class Reduced(Exception):
    def __init__(self, path):
        super().__init__()
        self.path = path

    def __reduce__(self):
        return type(self), (self.path,)


class InheritsReduce(Reduced):
    def __init__(self):
        super().__init__("fixed")


class LocalChild(Dropped):
    def __init__(self, path, line):
        super().__init__(path)


class Left(Dropped):
    pass


class Right(Dropped):
    pass


class Diamond(Left, Right):
    pass


class BelowDiamond(Diamond):
    def __init__(self, path, line):
        super().__init__(path)


class NotAnException:
    def __init__(self):
        super().__init__()


class Unpacked(Exception):
    def __init__(self, path, line):
        super().__init__(path, **{"line": line})


class Ranged(Exception):
    def __init__(self, path, line, column=0):
        super().__init__(path)


class Nested(Exception):
    def __init__(self, path, line):
        def describe():
            return f"{path}:{line}"

        super().__init__(describe())
