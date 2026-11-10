[walrus-candidate](../src/rules/logic/walrus_candidate.md) no longer inlines a name that `locals()`, `vars()`, `eval` or
`exec` reads in its function, or that `globals()` reads at module level, since the call would no longer find it.
