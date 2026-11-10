[`missing-final`](../src/rules/typing/missing_final.md) no longer suggests `Final` for a module name that a function
writes through `globals()`, or for any name when a computed write through `globals()` may set one.
