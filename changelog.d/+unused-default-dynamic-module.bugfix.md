[`unused-default`](../src/rules/logic/unused_default.md) leaves alone the functions of a module the run reads through
`getattr(module, name)` with a computed name or through its `__all__`, since its fix broke calls such as
`getattr(platformdirs, name)()`.
