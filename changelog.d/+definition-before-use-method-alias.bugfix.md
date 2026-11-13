[`definition-before-use`](../src/rules/layout/definition_before_use.md) no longer takes a class body that reads a
method, as `__str__ = __repr__` or `property(_get)` do, for one that calls it, which kept the method's helpers in place.
