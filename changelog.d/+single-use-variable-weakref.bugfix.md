[`single-use-variable`](../src/rules/logic/single_use_variable.md) no longer reports a name bound to a weak reference or
proxy. Inlining `ref = weakref.ref(obj)` into its one use lets the reference die at once, so code that counts weak
references, as pycparser's tests do, sees none.
