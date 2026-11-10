[`unused-export`](../src/rules/api/unused_export.md) no longer reports the names of an `__all__` that another module
imports or reads, as `from module import __all__` or `module.__all__` does. Such a module uses every name the list
holds, so `--fix` no longer removes them.
