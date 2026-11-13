[`definition-before-use`](../src/rules/layout/definition_before_use.md) no longer pins a class named in the annotations
of a standard decorator that never evaluates them, such as `dataclass`, `property`, `contextmanager` or
`functools.cache`, when annotations are lazy, and no longer reads `@name.setter` as calling the getter while the class
body runs.
