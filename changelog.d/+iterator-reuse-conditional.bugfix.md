[`iterator-reuse`](../src/rules/logic/iterator_reuse.md) reads the two sides of `a if test else b` as separate branches,
so a generator each side consumes is no longer reported.
