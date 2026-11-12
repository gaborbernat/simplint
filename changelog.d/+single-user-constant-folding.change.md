[`single-user-constant`](../src/rules/layout/single_user_constant.md) no longer reports a list, set or dict display, or
a `frozenset` call, since inlining one rebuilds it on every call. It reports a binary operation only where CPython folds
it at compile time, within CPython 3.11's limits: an integer result of 128 bits, a string or bytes repetition of 4096
characters, and a tuple repetition of 256 items, with nested items counted up to 1024. A division by zero or a type
error is left to run per call.
