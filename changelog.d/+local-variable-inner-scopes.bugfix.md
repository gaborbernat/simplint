[`single-use-variable`](../src/rules/logic/single_use_variable.md), [`dead-store`](../src/rules/logic/dead_store.md),
[`early-assignment`](../src/rules/logic/early_assignment.md) and
[`walrus-candidate`](../src/rules/logic/walrus_candidate.md) no longer take a variable of the same name that a nested
function, lambda or comprehension binds for itself as a read of the outer one, so they report the outer variable.
