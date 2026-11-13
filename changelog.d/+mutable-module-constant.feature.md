[`mutable-module-constant`](../src/rules/typing/mutable_module_constant.md) reports an `UPPER_CASE` or `Final` module
constant bound to a list, set or dict that no code changes, and its unsafe fix makes it a tuple, a frozenset, or a
`frozendict` from Python 3.15 and a `MappingProxyType` before it, retyping its annotation to match. The fix stays off
when a read in the module depends on the type, such as `==` with a list or `+`.
