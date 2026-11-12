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


class NotAnException:
    def __init__(self):
        super().__init__()
