[`all-format`](../src/rules/api/all_format.md) keeps reporting a tuple `__all__` but no longer offers its fix when a
test module mentions `__all__` or another module imports it: anyio's suite reads every `__all__` through `getattr` and
expects a tuple.
