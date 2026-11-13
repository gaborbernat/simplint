[`unused-export`](../src/rules/api/unused_export.md) and [`missing-from-all`](../src/rules/api/missing_from_all.md)
offer no fix when code outside the tests reads `__all__` off a module it imports by a computed name, as Pygments' lexer
loader does, since editing the list changed what that loader loads.
