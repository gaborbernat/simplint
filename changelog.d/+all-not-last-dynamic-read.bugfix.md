[`all-not-last`](../src/rules/api/all_not_last.md) no longer moves `__all__` past a later run-time read of it, such as
`sys.modules[__name__].__all__` or `globals()`. The report stays, but the fix is withheld.
