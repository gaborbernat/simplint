`string-concatenation` (SMP503) wraps a generator passed bare to `str`, `repr` or `ascii` in parentheses, so its fix no
longer writes a syntax error. A name bound by a PEP 695 type parameter is no longer treated as a `str`, so its chain is
reported without a fix.
