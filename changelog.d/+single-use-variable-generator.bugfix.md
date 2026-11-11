[`single-use-variable`](../src/rules/logic/single_use_variable.md) reports `x = ...; return x` in a generator again: it
left the case to Ruff's RET504, which skips generators, so neither tool reported it.
