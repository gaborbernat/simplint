[`definition-before-use`](../src/rules/layout/definition_before_use.md) no longer pins a class named in a standard
`dataclass` field when annotations are lazy, as `dataclass` reads them as strings or forward references, so the class
can sit with its caller's other helpers.
