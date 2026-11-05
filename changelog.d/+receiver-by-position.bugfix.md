`definition-before-use` (SMP201) reads a method's first parameter as the instance or class whatever its name, so
`klass._make()` in a classmethod counts as a use, as do `klass().load()` and a local that a method assigns from
`klass()` or `super().__new__(klass)`. A lambda in a class body, as `property(lambda self: self._load())`, reads its
first parameter the same way. A `staticmethod` has no such parameter, so `self` or `cls` in one no longer counts as a
use.
