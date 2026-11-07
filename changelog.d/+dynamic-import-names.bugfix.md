`unexported-public-name` and `module-variable-prefix` no longer rename a name that a module reads off the result of
`importlib.import_module`, `__import__` or `pytest.importorskip` called with a string literal, directly or through a
variable.
