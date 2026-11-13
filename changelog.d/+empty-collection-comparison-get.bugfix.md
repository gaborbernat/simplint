[`empty-collection-comparison`](../src/rules/logic/empty_collection_comparison.md) leaves `.get(key) == []`,
`.get(key, None) == []` and `getattr(obj, name, None) == ()` alone: a missing value gives `None`, which a truth test
would take for an empty one.
