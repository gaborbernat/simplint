Checking whether the documentation names a definition in a file outside any package, as in a `tests/` or `scripts/`
directory without `__init__.py`, no longer builds a string for each documented name: 10,000 such definitions against
20,000 documented names take 0.11 seconds instead of 0.6.
