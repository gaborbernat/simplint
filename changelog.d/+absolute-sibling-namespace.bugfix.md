`absolute-sibling-import` (SMP110) no longer reports an import in a directory without an `__init__.py` that Ruff's
`namespace-packages` makes a package, such as a pytest `tests` directory, where the relative import would fail.
