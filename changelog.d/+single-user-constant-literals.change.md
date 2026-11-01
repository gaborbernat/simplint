`single-user-constant` (SMP050) reports only constants whose value is written out in the source: strings, bytes,
numbers, booleans, `None`, operations between them, `frozenset` of them, and tuple, list, set and dict displays of them.
A call result such as a cache, a logger, a compiled pattern, a path or a client, and a name or attribute read from
elsewhere, stays at module level, since moving it would build a new object on every call and break tests that patch it.
