[`iterator-reuse`](../src/rules/logic/iterator_reuse.md) reports a `map()`, `zip()`, generator expression or other
one-pass iterator read a second time, or read inside a loop, where the read finds it empty without an error.
