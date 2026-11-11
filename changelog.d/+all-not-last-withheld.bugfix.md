[`all-not-last`](../src/rules/api/all_not_last.md) no longer moves `__all__` when a function or lambda reads it at run
time, such as through `globals()`, `"__all__"` or `sys.modules[__name__].__all__`, or when a comment sits above it or
above its `# fmt: off` pair. The report stays, but the fix is withheld.
