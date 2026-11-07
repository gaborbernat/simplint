`unused-export` (SMP402), `unexported-public-name` (SMP403), `unused-definition` (SMP409) and `test-only-code` (SMP408)
no longer report the `coverage_init` function of a module that `[tool.coverage.run] plugins` in `pyproject.toml` lists,
since coverage calls it by name. The module is found in the project's directory or in a `pythonpath` (pytest) or
`extra-paths` (ty) directory.
